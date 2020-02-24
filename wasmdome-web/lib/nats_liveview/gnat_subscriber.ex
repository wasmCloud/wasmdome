defmodule NatsLiveview.GnatSubscriber do
  use GenServer
  require Logger

  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts)
  end

  def init(topic: topic) do
    Gnat.sub(Gnat, self(), "#{topic}")
  end

  # Rebroadcast the message into Phoenix's PubSub
  def handle_info({:msg, msg = %{body: body, topic: topic}}, state) do
    Logger.debug("Received gnat message: #{inspect msg}")

    # This is going to go to every node in the cluster, so you may only want a single subscriber, or
    # have the subscription per-node and use local broadcast instead
    :ok = NatsLiveviewWeb.Endpoint.broadcast("gnat:#{topic}", "gnat_msg", %{body: body, topic: topic})

    {:noreply, state}
  end
end
