key_binder
==========

A small program to bind a key to an executable. Key presses activate and kill the spawned process.

User needs input group privs on debian. Use evtest to show the input key codes eg 59 for F1 key.

Requires libinput, libinput-dev, libudev, libudev-dev, pkg-config.
