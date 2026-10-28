# Wall mode

`/wall` shows the console's own board, full screen, for a monitor that stays on in
a room. It refreshes by itself and keeps the screen awake.

From the top:

- the **verdict** in a few words ("All good", "3 problems") with the date and
  the clock, and when the data was last updated;
- the **five figures** of the overview: monitors up, availability over 30 days,
  p95 response time, open alerts and mean time to resolve;
- the **health river**, the large picture of the board: one strip per monitor,
  worst on top;
- the **open alerts**, most severe first;
- **who is on call** now, with the schedule and the end of the shift.

There is no music and no widgets: black ground, the console's own type, states
written next to a shape and never by colour alone.

## Opening and leaving

Open it with the **Wall mode** icon in the top bar, from the command palette
("Wall mode") or by typing the URL. ++esc++ or **Exit** returns to the console.
An account limited to some teams or clients does not get the wall, since it
shows the whole fleet.

## Backdrop

The wall shares the console's dark look. The **Backdrop** button (top right,
next to Exit) can draw one of the [status pages](status-pages.md)' night cities
behind the verdict and the clock: Paris, New York, Tokyo or Dubai, or *Auto* for
the closest to the browser's time zone. It is off by default and remembered per
browser; `?city=paris` (or `newyork`, `tokyo`, `dubai`, `none`) on a link
chooses it for that display only. The sky follows the board: stars when all is
well, clouds on an advisory, a storm on a warning. On a phone, or while the tab
is hidden, the city holds still. A saved or linked `zermatt` now lands on Dubai.
