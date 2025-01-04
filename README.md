Fire Emblem preset:
0: map
1: battle

map-battle-map

s0|0:w50000-80000:1|1:w10000-15000:0

Balatro preset:
0: play
1: shop
2: booster pack

play-shop-(booster-shop-booster)-shop-play

s0|0:w120000-150000:1|1:w15000-60000;p30:2/1:w5000-10000;p70:0|2:w6000-20000:1

split by |
  first is s(\d+) [START]
  then for each of the rest, split by /
    for each split by :
      first is (\d+) [INDEX FROM]
      second split by ;
        for each is either
          w(\d+)(-(\d+))? [WAIT DEFINITION]
          p(\d+)(-(\d+))? [PROBABILITY DEFINITION]
      third is (\d+) [INDEX TO]
