# zawnabout-playbooks

Scripted headless runs that check the game end to end, such as flying somewhere, landing, or riding a vehicle a full lap. Run one with:

```
target/release/zawnabout --headless --script zawnabout-playbooks/NAME.txt --out DIR
```

It exits with 1 if any `expect` fails. Name vehicles (`ride tour`, `land-on ferry.1`), not numbers, because the numbers shift when routes change.
