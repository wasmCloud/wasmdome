defmodule NatsLiveviewWeb.Router do
  use NatsLiveviewWeb, :router

  pipeline :browser do
    plug :accepts, ["html"]
    plug :fetch_session
    plug :fetch_live_flash
    plug :protect_from_forgery
    plug :put_secure_browser_headers
  end

  pipeline :api do
    plug :accepts, ["json"]
  end

  scope "/", NatsLiveviewWeb do
    pipe_through :browser

    live "/", NatLive
  end

  # Other scopes may use custom stacks.
  # scope "/api", NatsLiveviewWeb do
  #   pipe_through :api
  # end
end
