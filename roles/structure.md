# Stratum V2(SV2) Roles Structure

Roles workspace is a collection of crates that utilizes SV2 protocols and sub-protocols to build a
specific role in the SV2 ecosystem. For example, a Pool or Job Declarator role.

While this document will draw the general structure of the roles workspaces, the main focus is to
explain the internal connections or communication layer between the roles/crates and how they
interact with each other.

The workspace consists of five different crates, it is possible to utilize a single role or
multiple, depending on the use case. All crates are built in a library-first approach but are also
accessible as a binary. So you can either use the library in your code or run the binary to start a
role.

In a mining setup, each role but the Template Provider can either be a downstream or upstream role,
depending on the communication direction.  For example, a Pool is an upstream role for a Translator
and a downstream for a Template Provider. When a role is a downstream role, it serves as a client to
the upstream role, or we can call it a server. Generally, this workspace uses `tokio` crate for
networking and in order to start a role as a server, a socket is bound to a specific address and
port. When a role is a client, it connects to the server role by using the server's address and
port.

When two roles connect to each other, they first perform a handshake to establish an encrypted
connection. And then they start to communicate via specific SV2 messages. The first SV2 message is
sent by the downstream role, `SetupConnection` message. The upstream role responds with either
`SetupConnectionSuccess` or `SetupConnectionError` message. Subsequent to the successful setup,
**roles will utilize sub/pub channels to manage the state of the connection**.  For example, on a
connection between a Pool and a Template Provider, the Template Provider is expected to send
`SetNewPrevHash` message. The Pool crate defines a specific channel to manage the state of this
message, i.e., each time `SetNewPrevHash` is received through a network stream, the Pool will
publish this message to the channel and a subscriber is expected to handle it.

The following is the list of crates along with a brief description and a look into their channel(s)
structure and usage.

All of the these
channels will be mentioned for each role in the following section.

* **Template Provider**:

  TODO - Not a crate, fork of bitcoin core.

  pub/sub channels:
  * `message`: A new message about...

* **Job Declarator Client(JD-Client)**:

  TODO - Used on miner side to enable job declaration.

  pub/sub channels:
  * `message`: A new message about...

* **Job Declarator Server(JD-Server)**:

  TODO - Used on pool side to enable job declaration.

  pub/sub channels:
  * `message`: A new message about...

* **Mining Proxy**:

  TODO - SV2 aggregator

  pub/sub channels:
  * `message`: A new message about...

* **Pool**:

  TODO - SV2 pool

  pub/sub channels:
  * `message`: A new message about...

* **Translator**:

  TODO - Used by SV1 miners to connect to SV2 pools.

  pub/sub channels:
  * `message`: A new message about...

