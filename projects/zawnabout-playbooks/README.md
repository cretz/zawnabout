# zawnabout-playbooks

Scripted headless runs that check the game end to end, such as flying somewhere, landing, or riding a vehicle a full lap. Run one with:

```
target/release/zawnabout --headless --script zawnabout-playbooks/NAME.txt --out DIR
```

It exits with 1 if any `expect` fails. Cards for things found are logged and dismissed so runs carry on, unless the playbook says `cards keep`. Name vehicles (`ride tour`, `land-on ferry.1`), not numbers, because the numbers shift when routes change.

Gameplay playbooks: `fly-courses`, `fly-moves` and `fly-rides` are made from `play.kdl` (every gate flown through with `try-gate`, every move, every ride) and need remaking when it changes. Checks: `flown ID` (stunt done, course flown or race won), `gate COURSE N`, `racing`. Commands: `try-gate ID [N]`, `goto-race ID start|end`. `set-effects` finishes both sets and talks to someone they bring. `story-lights` takes on Tobin's story, lights every lamp, and sets it aside part way (`story ID start|stop`, `goto-lamp K`, checks `dark`, `story-done ID`). `cards auto` goes back to dismissing cards after `cards keep`. The stadium goals in `fly-courses` are walked through, being too low to fly. `give-ride` asks the passenger (`ask-rider`, check `say` for their bubble). `goto-find` also takes postcard ids. Debug: `best-view ID` (where a postcard's picture would be taken from), `rider-bits`, `hide-mesh N...` (native only: statics plus 1000000, all water 3000000, all terrain 4000000).
