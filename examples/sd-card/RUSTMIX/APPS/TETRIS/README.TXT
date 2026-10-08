Rustmix-Wave native Tetris sample: a mode list with Zen and Classic.

Zen has no gravity: the piece moves only when you press,
which suits the e-paper panel and the battery.
Classic falls one row a second, so the panel refreshes on its own;
locked pieces still follow the same refresh rules.

  UP    move left
  DOWN  move right
  SELECT rotate
  BOOT (short) drop the piece

The dotted shadow shows where the piece will land.
A full refresh every 20 locked pieces clears ghosting.
After game over, SELECT starts a new game.

Each mode keeps its own best score, saved to RECORDS.TXT when you leave.
