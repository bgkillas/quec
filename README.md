# quec
[![crates.io](https://img.shields.io/crates/v/quec.svg)](https://crates.io/crates/quec) [![AUR](https://img.shields.io/aur/version/quec.svg)](https://aur.archlinux.org/packages/quec/)

history files located ``$HOME/.quec/`` ``C:\\Users\\USERNAME\\AppData\\Roaming\\quec``
## wont fix issues
non-ascii characters that are not 1 char long messes stuff up

0 width ascii characters are deleted

tab looks like space

windows files are converted to linux

requires modern terminal on windows, like windows terminal
## keybindings
``i`` to enter edit mode

``esc`` to exit edit mode

``h`` left

``l`` right

``j`` down

``k`` up

``0`` move to beginning of line

``$`` move to end of line

``page down`` move 1 page down

``page up`` move 1 page up

``home`` go to start of file

``end`` go to end of file

``y`` to copy line

``d`` to cut line

``p`` to print line

``w`` to save

`` ` `` to go to next file

``~`` to go to last file

``q`` to quit

``u`` to undo

``U`` to redo

``/`` to start search mode

search mode:

``esc`` to quit search mode

``enter`` to search through file