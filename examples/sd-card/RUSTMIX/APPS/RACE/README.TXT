Rustmix-Wave native Race sample: a mode list with Classic and Zen.

The road runs from right to left: cars and cones arrive in the last
column and move one column toward you each step, 20 metres a step.
Each new column leaves a gap in or next to the gap before it, so a
careful driver can always get through.

  UP    change a lane up
  DOWN  change a lane down
  SELECT nitro (Classic) or step (Zen)
  BOOT (short) pause (Classic) or step (Zen)

Classic steps once a second at speed 1 and takes 100 ms off each
kilometre, down to 500 ms. Nitro starts with one charge, gains one
each kilometre (three at most) and spends one for three steps at once.
Zen has no timer: the road moves only when you press.

Other cars are outlines; your car is solid. A full refresh every
30 steps clears ghosting. After a crash, SELECT runs the highlighted
row: Race again, Mode or Back to Games.

Each mode keeps its own best distance in metres, saved to RECORDS.TXT
when you leave.
