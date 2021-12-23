### hypnos core

===================

note: this is highly unstable in it's current state, do NOT use

hypnos core is an attempt to create a server managment tool in the form of a discord bot

#### current features
* self-recompiling (can recieve upstream updates and recompile itself)
* chat bridge between discord <-> mc (will support multiple servers soon)
* cross-game chat-bridge, due to the bot structure other games are supported
* baked in multithreading, each thread has a different job that is kept in sync
* execute mc commands, shell commands, etc, via discord
* math eval, example: `=4*4` will return `-> 16`

#### currently under development
* server monitor (display ram usage, disk usage, etc.)
* backup manager
* improving reliable recompiling

#### future features
* unscheduled backup managment (creation/deletion)
* discord auto mod/ban features
* interlinked servers, the bot can communicate with other copies of itself on other servers
