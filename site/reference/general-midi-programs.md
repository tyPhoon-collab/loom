# General MIDI Program Numbers

Loom sends Program Change values as raw MIDI program numbers.

```loom
# Piano: 1
## pc 0
```

Most General MIDI tables are written as `1..128`. Loom uses the MIDI wire value
`0..127`, so subtract `1` from a General MIDI program number when writing
`## pc`.

| Loom `## pc` | GM number | Sound |
| ---: | ---: | --- |
| 0 | 1 | Acoustic Grand Piano |
| 1 | 2 | Bright Acoustic Piano |
| 2 | 3 | Electric Grand Piano |
| 3 | 4 | Honky-tonk Piano |
| 4 | 5 | Electric Piano 1 |
| 5 | 6 | Electric Piano 2 |
| 6 | 7 | Harpsichord |
| 7 | 8 | Clavinet |
| 8 | 9 | Celesta |
| 9 | 10 | Glockenspiel |
| 10 | 11 | Music Box |
| 11 | 12 | Vibraphone |
| 12 | 13 | Marimba |
| 13 | 14 | Xylophone |
| 14 | 15 | Tubular Bells |
| 15 | 16 | Dulcimer |
| 16 | 17 | Drawbar Organ |
| 17 | 18 | Percussive Organ |
| 18 | 19 | Rock Organ |
| 19 | 20 | Church Organ |
| 20 | 21 | Reed Organ |
| 21 | 22 | Accordion |
| 22 | 23 | Harmonica |
| 23 | 24 | Tango Accordion |
| 24 | 25 | Acoustic Guitar (nylon) |
| 25 | 26 | Acoustic Guitar (steel) |
| 26 | 27 | Electric Guitar (jazz) |
| 27 | 28 | Electric Guitar (clean) |
| 28 | 29 | Electric Guitar (muted) |
| 29 | 30 | Overdriven Guitar |
| 30 | 31 | Distortion Guitar |
| 31 | 32 | Guitar Harmonics |
| 32 | 33 | Acoustic Bass |
| 33 | 34 | Electric Bass (finger) |
| 34 | 35 | Electric Bass (pick) |
| 35 | 36 | Fretless Bass |
| 36 | 37 | Slap Bass 1 |
| 37 | 38 | Slap Bass 2 |
| 38 | 39 | Synth Bass 1 |
| 39 | 40 | Synth Bass 2 |
| 40 | 41 | Violin |
| 41 | 42 | Viola |
| 42 | 43 | Cello |
| 43 | 44 | Contrabass |
| 44 | 45 | Tremolo Strings |
| 45 | 46 | Pizzicato Strings |
| 46 | 47 | Orchestral Harp |
| 47 | 48 | Timpani |
| 48 | 49 | String Ensemble 1 |
| 49 | 50 | String Ensemble 2 |
| 50 | 51 | Synth Strings 1 |
| 51 | 52 | Synth Strings 2 |
| 52 | 53 | Choir Aahs |
| 53 | 54 | Voice Oohs |
| 54 | 55 | Synth Voice |
| 55 | 56 | Orchestra Hit |
| 56 | 57 | Trumpet |
| 57 | 58 | Trombone |
| 58 | 59 | Tuba |
| 59 | 60 | Muted Trumpet |
| 60 | 61 | French Horn |
| 61 | 62 | Brass Section |
| 62 | 63 | Synth Brass 1 |
| 63 | 64 | Synth Brass 2 |
| 64 | 65 | Soprano Sax |
| 65 | 66 | Alto Sax |
| 66 | 67 | Tenor Sax |
| 67 | 68 | Baritone Sax |
| 68 | 69 | Oboe |
| 69 | 70 | English Horn |
| 70 | 71 | Bassoon |
| 71 | 72 | Clarinet |
| 72 | 73 | Piccolo |
| 73 | 74 | Flute |
| 74 | 75 | Recorder |
| 75 | 76 | Pan Flute |
| 76 | 77 | Blown Bottle |
| 77 | 78 | Shakuhachi |
| 78 | 79 | Whistle |
| 79 | 80 | Ocarina |
| 80 | 81 | Lead 1 (square) |
| 81 | 82 | Lead 2 (sawtooth) |
| 82 | 83 | Lead 3 (calliope) |
| 83 | 84 | Lead 4 (chiff) |
| 84 | 85 | Lead 5 (charang) |
| 85 | 86 | Lead 6 (voice) |
| 86 | 87 | Lead 7 (fifths) |
| 87 | 88 | Lead 8 (bass + lead) |
| 88 | 89 | Pad 1 (new age) |
| 89 | 90 | Pad 2 (warm) |
| 90 | 91 | Pad 3 (polysynth) |
| 91 | 92 | Pad 4 (choir) |
| 92 | 93 | Pad 5 (bowed) |
| 93 | 94 | Pad 6 (metallic) |
| 94 | 95 | Pad 7 (halo) |
| 95 | 96 | Pad 8 (sweep) |
| 96 | 97 | FX 1 (rain) |
| 97 | 98 | FX 2 (soundtrack) |
| 98 | 99 | FX 3 (crystal) |
| 99 | 100 | FX 4 (atmosphere) |
| 100 | 101 | FX 5 (brightness) |
| 101 | 102 | FX 6 (goblins) |
| 102 | 103 | FX 7 (echoes) |
| 103 | 104 | FX 8 (sci-fi) |
| 104 | 105 | Sitar |
| 105 | 106 | Banjo |
| 106 | 107 | Shamisen |
| 107 | 108 | Koto |
| 108 | 109 | Kalimba |
| 109 | 110 | Bag Pipe |
| 110 | 111 | Fiddle |
| 111 | 112 | Shanai |
| 112 | 113 | Tinkle Bell |
| 113 | 114 | Agogo |
| 114 | 115 | Steel Drums |
| 115 | 116 | Woodblock |
| 116 | 117 | Taiko Drum |
| 117 | 118 | Melodic Tom |
| 118 | 119 | Synth Drum |
| 119 | 120 | Reverse Cymbal |
| 120 | 121 | Guitar Fret Noise |
| 121 | 122 | Breath Noise |
| 122 | 123 | Seashore |
| 123 | 124 | Bird Tweet |
| 124 | 125 | Telephone Ring |
| 125 | 126 | Helicopter |
| 126 | 127 | Applause |
| 127 | 128 | Gunshot |

Channel 10 is conventionally used for General MIDI percussion. Its drum sounds
are selected by MIDI note number, not by Program Change.
