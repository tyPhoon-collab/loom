# Playback

Loom does not include a synthesizer. It sends MIDI events to an output port, and
another app turns those MIDI events into sound.

The recommended local setup is FluidSynth with a General MIDI SoundFont.

## FluidSynth with Nix

If you already have a SoundFont installed, start FluidSynth like this:

```bash
nix run nixpkgs#fluidsynth -- -a coreaudio -m coremidi ~/Library/Audio/Sounds/Banks/FluidR3_GM.sf2
```

For debugging, add `-v` to print incoming MIDI events:

```bash
nix run nixpkgs#fluidsynth -- -a coreaudio -m coremidi -v ~/Library/Audio/Sounds/Banks/FluidR3_GM.sf2
```

`nix run nixpkgs#fluidsynth` already launches the `fluidsynth` binary. Do not
repeat `fluidsynth` after `--`.

## FluidSynth with Homebrew

Install FluidSynth:

```bash
brew install fluid-synth
```

Then start it with the same Core Audio and Core MIDI drivers:

```bash
fluidsynth -a coreaudio -m coremidi ~/Library/Audio/Sounds/Banks/FluidR3_GM.sf2
```

For debugging:

```bash
fluidsynth -a coreaudio -m coremidi -v ~/Library/Audio/Sounds/Banks/FluidR3_GM.sf2
```

Adjust the SoundFont path if your `.sf2` file is somewhere else.

## Play from Loom

In another terminal, list MIDI output ports:

```bash
loom ports
```

Pick the FluidSynth port index, then play or live-code:

```bash
loom play examples/starter/melody-simple.loom --port 0
loom live examples/starter/melody-simple.loom --port 0
```

If the FluidSynth port is not listed, restart FluidSynth and run `loom ports`
again.

## MIDI Monitor

If sound does not play, use MIDI Monitor to check whether Loom is sending MIDI
events. It is useful for verifying note, channel, control change, and program
change messages before debugging the synth or audio output.
