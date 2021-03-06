# Credentials/OTT Minter

This is a simple (non-actor) service that sits on certain NATS subjects and dispenses one-time-tokens (OTT). These tokens are requested by the wasmdome website by individual users. If a user gets one of these tokens, they then have a short period of time in which to switch over to the wasmdome tooling to "claim" their credentials.

An OTT (such as `TAMHDMVCV3FNRJ29MGPP31`) can be exchanged for a set of short-lived credentials signed by the `WASMDOME` [NGS](https://synadia.com/ngs/) account. This new set of credentials is to be used by a leaf node NATS server allowing a developer to connect to NGS and join the _global lattice_ for the next arena matchup. The credentials will expire shortly after the match is scheduled to be finished.

The credentials will default to expiring 20 minutes after they are minted, so the website should only start dispensing OTTs within ~15 minutes of the next upcoming match.

Note that the website should not allow the dispensation of OTTs if:

* It isn't within some short period time prior to the next scheduled match
* The site has determined that there are no more available slots in the upcoming match

**NOTE** The account for which a one-time token is generated must also be the same account ID used in the request to convert that token into credentials. Put another way, an attacker must possess both the public key of the developer's signing account (only visible _to that user_ on their profile page) and the OTT generated by that user.

The Wasmdome website will emit a `wasmdome` command line that the developer can simply copy and paste in order to convert the token into credentials.
