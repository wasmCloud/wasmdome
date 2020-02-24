# NatsLiveview

For Kevin on Twitter.

* https://github.com/sb8244/gnat-live-view-demo/blob/master/lib/nats_liveview/gnat_subscriber.ex
* https://github.com/sb8244/gnat-live-view-demo/blob/master/lib/nats_liveview_web/lives/nat_live.ex#L24

I used the less efficient way of handling the LiveView (each message is stored in the LiveView). You
could implement pruning, or use LiveView temporary_assigns + prepend message for the same effect without
the infinitely growing process memory.

# Usage

To start your Phoenix server:

  * Install dependencies with `mix deps.get`
  * Install Node.js dependencies with `cd assets && npm install`
  * Start Phoenix endpoint with `mix phx.server`

Now you can visit [`localhost:4000`](http://localhost:4000) from your browser.
