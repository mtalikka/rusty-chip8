# What?

This is an implementation of the CHIP-8 interpreted language. It attempts to emulate a basic virtual machine which can run binary applications written in said language. Specificiations taken from [here](http://devernay.free.fr/hacks/chip8/C8TECH10.HTM).

# Design

The main component is the library, chip8_lib, which is standalone so that it may be plugged into multiple frontends. Development is currently targeting a desktop environment, but the idea is to eventually get it working as an embedded application.

# Build status

[![windows](https://github.com/mtalikka/rusty-chip8/actions/workflows/windows.yml/badge.svg)](https://github.com/mtalikka/rusty-chip8/actions/workflows/windows.yml)
[![ubuntu](https://github.com/mtalikka/rusty-chip8/actions/workflows/ubuntu.yml/badge.svg)](https://github.com/mtalikka/rusty-chip8/actions/workflows/ubuntu.yml)
[![macos](https://github.com/mtalikka/rusty-chip8/actions/workflows/macos.yml/badge.svg)](https://github.com/mtalikka/rusty-chip8/actions/workflows/macos.yml)
