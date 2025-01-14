# Feusic
Listen to music like you're playing a game.

Many games have a soundtrack with multiple musics crossfading between each other based
on what's happening in the game (e.g. Fire Emblem, Balatro).
Feusic is a data structure that defines the same behaviour.

Feusic can be played using Feusic Player, a music player that reads Feusics in addition to standard music files.

### Feusic Data Structure
Feusic is a data structure that defines a list of musics and how they crossfade between each other.
It is defined in a TOML file.

```toml
timing = "s0|0:w50000-80000:1|1:w10000-15000:0" # Defines the timing of the music.
duration = 600 # Defines the duration in seconds before removing the loop.
loop_start = 2.5 # Optional. Defines the start of the loop in seconds. Default is 0.
loop_end = 114.5 # Optional. Defines the end of the loop in seconds. Default is end of music.
```

### Timing
#### Example:
`s0|0:w120000-150000:1|1:w15000-60000;p30:2/w5000-10000;p70:0|2:w6000-20000:1`

#### Rules:
split by |
  first is s(\d+) [INDEX OF START MUSIC]
  then for each of the rest
    first is (\d+): [INDEX FROM]
    then for the rest split by /
      for each split by :
        first split by ;
          for each is either
            w(\d+)(-(\d+))? [WAIT DEFINITION]
            p(\d+)(-(\d+))? [PROBABILITY DEFINITION]
        second is (\d+) [INDEX TO]
