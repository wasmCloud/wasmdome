# Kode Frieze

Kode Frieze is one of Boylur's henchmen, ready to take out player mechs in the `wasmdome`.

His strategy is simple: proceed to the center of the arena, and destroy any mech that comes near him. Don't modify any production code near this mech!

Around the code, you might see an odd snippet:
```
vec![mech.request_radar(), mech.register_acc(ECX, 1)]
```

This is us making use of the counter register in order to know how many turns `kode-frieze` does nothing. If he is sitting at the center of the arena and doesn't fire for say, 5 turns, we could examine the value in the counter register and choose to move to a different part of the arena to find enemies. This is not implemeneted, but just an example on how a mech could be designed to prevent inactivity.