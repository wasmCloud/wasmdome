defmodule NatsLiveviewWeb.NatLive do
  use Phoenix.LiveView

  # You could use "prepend" in this type of feed, although each message needs to have
  # some unique ID (could be the received time?) and I didn't want to set that up.
  def render(assigns) do
    ~L"""
    <%= for message <- @messages do %>
      <pre><%= inspect message %></pre>
    <%= end %>
    """
  end

  def mount(_params, _session, socket) do
    if connected?(socket) do
      #Process.send_after(self(), :tick, 3_000)
      # TODO: this subscription model doesn't allow wildcards, so these need to be ad-hoc per metch
      :ok = Phoenix.PubSub.subscribe(NatsLiveview.PubSub, "gnat:wasmdome.match_events.45")
    end

    {:ok, assign(socket, :messages, [])}
  end

  # Broadcast messages will end up here
  def handle_info(%{event: "gnat_msg", payload: payload}, socket = %{assigns: %{messages: messages}}) do
    {:noreply, assign(socket, :messages, [payload | messages])}
  end

  #def handle_info(:tick, socket) do
  #  Process.send_after(self(), :tick, 3_000)
  #  :ok = Gnat.pub(Gnat, "pawnee", "Leslie Knope recalled from city council #{System.system_time(:second)}")
  #  {:noreply, socket}
  #end
end
