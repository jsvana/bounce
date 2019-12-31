# bounce

An attempt to create a more convenient IRC bouncer with a specific focus on message replay.

## Proposed Log Storage
```
  <username>/
    <server:hostport>/
      <channel>/
        <day_log>
        [...day_log]
```

```
      Read
      ───▶               ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐
                                                Bounce Server
      Write              │                                                         │
      ════▶
                         │                           ┌───────────────┐             │
         ╔═══════════════════════════════════════════│ foo.bar:1234  │◀═══════════════════════════╗
         ║               │                           └───────────────┘             │              ║
         ▼                                                                                        ║
┌────────────────┐       │       ┌────────────┐                                    │        ┌──────────┐
│     Actual     │               │    Ping    │      ┌───────────┐                          │ User 1's │
│  foo.bar:1234  │───────┼──────▶│(keepalive)?│─no──▶│    Log    │─────────────────┼───────▶│IRC client│
│     server     │               └────────────┘      └───────────┘                          └──────────┘
└────────────────┘       │              ║                                          │
         ▲                              ║
         ║               │              ║                                          │
         ╚═════════════════════════yes══╝
                         │                                                         │

                         │                                                         │

         ╔═══════════════╬═════════yes══╗                                          │
         ║                              ║
         ▼               │              ║                                          │
┌────────────────┐               ┌────────────┐                                            ┌──────────┐
│     Actual     │       │       │    Ping    │      ┌───────────┐                 │       │ User 2's │
│  foo.bar:1234  │──────────────▶│(keepalive)?│─no──▶│    Log    │────────────────────────▶│IRC client│
│     server     │       │       └────────────┘      └───────────┘                 │       └──────────┘
└────────────────┘                                                                               ║
         ▲               │                           ┌────────────────┐            │             ║
         ╚═══════════════════════════════════════════│  foo.bar:1234  │◀═════════════════════════╝
                         │                           └────────────────┘            │
                          ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
```

## Connections

A user can connect to multiple `server:hostport`s through `bounce`. Each user connection will become two: the "user-facing connection" between the user and `bounce`, and the "server-facing connection" between `bounce` and the actual IRC server.

In most cases messages will simply be proxied from the user through `bounce` to the destination IRC server. The exception to that rule is the `PING` message, which `bounce` will not forward and instead will reply automatically.

Messages sent from actual IRC servers will only be sent to users' IRC clients after the messages have been persisted to the log.

Each direction of communication will be a thread, so each user's `server:hostport` connection will consist of two threads.
