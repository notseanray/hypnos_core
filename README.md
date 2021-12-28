### hypnos core

===============

Hypnos core is an attempt to create a server managment tool in the form of a discord bot, it has many features geared towards Minecraft, however, it is very flexible and can be adapted for any game server using a command line interface.

The end goal of this project is for me to improve my programming skills, if you see any code that is horrible please let me know so I can improve myself.

This is not intended for anyone else to use.

#### current features
* self-recompiling, can recieve upstream updates and recompile itself
* unified chat bridge between minecraft, discord, and other games
* async code base
* execute in-game commands, shell commands, etc, via discord
* in-game math eval, example: `=4*4` will return `-> 16`
* server monitor, checks server health and warns if there are issues

#### currently under development
* backup manager
* improving reliable recompiling

#### future features
* unscheduled backup managment (creation/deletion)
* discord auto mod/ban features
* copies of itself can communicate without discord
