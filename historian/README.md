# Historian

The historian is responsible for listening to the event streams for matches and recording them in an append-only event stream/store. It can also handle requests to replay the event history for a match, allowing web and other applications to browse through match history long after it finished.

**NOTE** It's important to _not_ use the same topic the "live" match events occur on for the replay. This is because there are event sourcing aggregates listening on the live subject to do things like compute leaderboard scores and trigger other events.

Important Message Broker Subjects:

* `wasmdome.match.*.events` - Historian **must** subscribe to this to record match events to the historical stream
* `wasmdome.history.replay` - Historian **must** subscribe to this to answer requests for historial replays
* `wasmdome.match.{}.events.replay` - Historian publishes to this subject upon request for a replay
