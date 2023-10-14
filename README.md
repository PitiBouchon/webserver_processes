# Webserver Processes

Simple web server that lists the processes running

## HTTP Requests

- `POST /acquire_process_list`: Update the list of processes
- `GET /processes`: Return the list of processes in a JSON format
- `GET /search`: Return the list of processes in a JSON format with filtering using query string parameters:
    - `pid=<num>`: filter on pid
    - `username=<str>`: filter on username
  
  Both parameters are optional, and if combine act as an AND
- `GET /data`: SSE endpoint that return new processes found by `/acquire_process_list`

## Example

A process should look like:
```json
[
  {
    "pid": 674,
    "name": "synapse",
    "uid": 100,
    "username": "user"
  }
]
```

*(On Windows the **uid** is a string because it is a [sid](https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/manage/understand-security-identifiers))*

### Curl commands

- `curl -X POST http://localhost:8080/acquire_process_list` (or `curl -Method POST http://localhost:8080/acquire_process_list` on windows if `curl` is an alias to `Invoke-WebRequest`)
- `curl http://localhost:8080/processes`
- `curl -G -d 'pid=<num>' -d 'username=<str>' http://localhost:8080/search` (or `curl -Body @{pid=<num>;username="<str>"} http://localhost:8080/search` on windows with `Invoke-WebRequest`)
- `curl http://localhost:8080/data`

## Dependencies

- **sysinfo**: used to get the list of processes running
- **axum**: the webserver framework
- tokio: the async runtime needed with axum
- tracing / tracing-subscriber: tracing with axum
- tokio-stream: used only for streaming the receiver of the broadcast (see also [Receiver](https://docs.rs/tokio/latest/tokio/sync/broadcast/struct.Receiver.html))
- futures: for the streaming traits
- serde: serialize / deserialize derive traits for the json format
