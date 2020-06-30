use nkeys::KeyPair;
use redis::{Commands, RedisResult};
use serde::{de::DeserializeOwned, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[macro_use]
extern crate serde;

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_json;

type Result<T> = ::std::result::Result<T, Box<dyn ::std::error::Error>>;

const SUBJECT_OTT_GEN: &str = "wasmdome.internal.ott.gen";
const SUBJECT_CREDS_GEN: &str = "wasmdome.public.creds.claim";
const REDIS_URL_ENV: &str = "REDIS_URL";
const SIGNING_KEY_ENV: &str = "SIGNING_KEY";
const OTT_EXPIRES_KEY_ENV: &str = "OTT_EXPIRES_SECONDS";
const OTT_DEFAULT_EXPIRES_SECONDS: &str = "300"; // 5 minutes
const QUEUE: &str = "credsminter";
const CREDS_EXPIRE_MINUTES: u64 = 20;

fn main() -> Result<()> {
    let _ = env_logger::builder().format_module_path(false).try_init();
    info!("Starting up Credentials/OTT minter");
    let signing_key = std::env::var(SIGNING_KEY_ENV).unwrap();

    let nc = nats::connect("127.0.0.1")?; // Connect to the leaf node on loopback
    let redis_url = std::env::var(REDIS_URL_ENV).unwrap_or("redis://0.0.0.0:6379".into());
    let r = redis_url.to_string();

    let ott_expiration_seconds: u32 = std::env::var(OTT_EXPIRES_KEY_ENV)
        .unwrap_or(OTT_DEFAULT_EXPIRES_SECONDS.to_string())
        .parse()?;

    let _sub = nc
        .queue_subscribe(SUBJECT_OTT_GEN, QUEUE)?
        .with_handler(move |msg| {
            let redisclient = redis::Client::open(redis_url.to_string()).unwrap();
            let mut con_a = redisclient.get_connection().unwrap();
            gen_ott(msg, &mut con_a, ott_expiration_seconds);
            Ok(())
        });

    let _sub2 = nc
        .queue_subscribe(SUBJECT_CREDS_GEN, QUEUE)?
        .with_handler(move |msg| {
            let redisclient = redis::Client::open(r.to_string()).unwrap();
            let mut con_b = redisclient.get_connection().unwrap();
            generate_creds(msg, &mut con_b, signing_key.to_string());
            Ok(())
        });

    ::std::thread::park();
    Ok(())
}

// Generate a one-time token, store it in the cache, and return it on the reply-to
fn gen_ott(msg: nats::Message, con: &mut redis::Connection, expiration_seconds: u32) {
    info!("Generating OTT");
    let req: TokenRequest = serde_json::from_slice(&msg.data).unwrap();
    let ott = nuid::next().to_string().to_uppercase();
    let _: bool = con
        .set_ex(ott_key(&ott), req.account_key, expiration_seconds as usize)
        .unwrap();

    msg.respond(ott.as_bytes()).unwrap();
}

fn ott_key(ott: &str) -> String {
    format!("wasmdome:ott:{}", ott)
}

fn generate_creds(msg: nats::Message, con: &mut redis::Connection, signing_key: String) {
    info!("Exchanging OTT for credentials");
    let req: CredentialsRequest = serde_json::from_slice(&msg.data).unwrap();
    let v: RedisResult<String> = con.get(ott_key(&req.token));
    let response = match v {
        Ok(ott_account) => {
            if ott_account != req.account_key {
                CredentialsResponse::Error(
                    "Attempt to exchange token for invalid account".to_string(),
                )
            } else {
                let (jwt, seed) = mint_creds(&signing_key).unwrap();
                let _: u32 = con.del(ott_key(&req.token)).unwrap();
                CredentialsResponse::Valid {
                    user_jwt: jwt,
                    user_secret: seed,
                }
            }
        }
        Err(_) => CredentialsResponse::Error("No such token".to_string()),
    };
    msg.respond(&serde_json::to_vec(&response).unwrap())
        .unwrap();
}

fn mint_creds(signing_key: &str) -> Result<(String, String)> {
    let signer = KeyPair::from_seed(signing_key)?;
    let user = KeyPair::new_user();
    let header = claims_header();
    let creds = gen_creds(&user.public_key(), &signer.public_key(), "Generated User");
    let encoded = encode(&header, &creds, &signer)?;
    Ok((encoded, user.seed()?))
}

fn gen_creds(sub: &str, iss: &str, name: &str) -> serde_json::Value {
    let exp = (60 * CREDS_EXPIRE_MINUTES) + since_the_epoch().as_secs(); // expire in 1 hour
    let nid = nuid::next().to_string();
    json!({
        "jti": nid,
        "exp": exp,
        "iat": since_the_epoch().as_secs(),
        "iss": iss.to_string(),
        "name": name.to_string(),
        "sub": sub.to_string(),
        "type": "user",
        "nats": {
            "pub": {
                "allow": [
                    "wasmbus.provider.wasmdome.engine.default", // Only allowed to communicate with the engine provider
                    "_INBOX.>",
                ]
            },
            "sub": {
                "allow": [
                    "wasmbus.actor.*", // The actor target subject on the bus
                    "INBOX.>"
                ]
            }
        }
    })
}

fn encode(header: &serde_json::Value, claims: &serde_json::Value, kp: &KeyPair) -> Result<String> {
    let jheader = to_jwt_segment(&header)?;
    let jclaims = to_jwt_segment(&claims)?;

    let head_and_claims = format!("{}.{}", jheader, jclaims);
    let sig = kp.sign(head_and_claims.as_bytes())?;
    let sig64 = base64::encode_config(&sig, base64::URL_SAFE_NO_PAD);
    Ok(format!("{}.{}", head_and_claims, sig64))
}

fn claims_header() -> serde_json::Value {
    json!({
      "typ": "jwt",
      "alg": "ed25519"
    })
}

fn since_the_epoch() -> Duration {
    let start = SystemTime::now();
    start
        .duration_since(UNIX_EPOCH)
        .expect("A timey wimey problem has occurred!")
}

fn to_jwt_segment<T: Serialize>(input: &T) -> Result<String> {
    let encoded = serde_json::to_string(input)?;
    Ok(base64::encode_config(
        encoded.as_bytes(),
        base64::URL_SAFE_NO_PAD,
    ))
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct TokenRequest {
    account_key: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct CredentialsRequest {
    account_key: String,
    token: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
enum CredentialsResponse {
    Valid {
        user_jwt: String,
        user_secret: String,
    },
    Error(String),
}
