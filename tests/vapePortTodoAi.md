# Vape randomization port

Each clicker has a manual selector: `citron`, `vape-normal`, `vape-extra`, `vape-extra+`.
All Vape choices use the existing min/max CPS range. The Humanize switch controls
only `citron`; its saved value is retained when switching modes. Old configs default
to `citron`. Existing recording fatigue can still replace the range with its 5–8 CPS
floor, and double clicking still adds its second click.

Source: [Vape-fabric at 4d4e26c](https://github.com/pvputils/Vape-fabric/tree/4d4e26ce92a341516446ad60220e460681710336).
The port follows `src/main/java/com/memeasaur/untitled/`:

- `VapeAutoClickerTimingTodoAi.java`: all three `n6.e()` branches.
- `VapeSilentProfileTodoAi.java`: Extra+'s `kB` backend.
- `VapeAutoClickerCycleTodoAi.java`: interval adjustment and signed release/hold split.

The original Java Random and SplittableRandom algorithms, endpoint integer casts,
exclusive lower CPS sample, integer division, counters, draw ordering, and profile
timers are preserved. Normal and Extra share their original state; Extra+ has its
own profile state. Each clicker owns a separate timing instance. Citron's input
gates, scheduler, sound, jitter, and native input remain responsible for execution.
The split is returned as release then hold to fit Citron's existing cycle order.
The port does not introduce Vape's activation delay or Minecraft/Fabric hooks.

The slider range is a policy input, not a guarantee of measured output CPS: the
source algorithms add pauses and fatigue and Extra+ scales its internal range by
1.5. Those behaviors are deliberately retained. Citron's existing scheduling and
Windows input delivery also affect measured output timing.

## Validation

Run `cargo test --offline`. The Java parity test checks 21,600 intervals across
36 traces: three modes, four ranges (including equal endpoints and CPS above 20),
and three seeds. Scripted gaps exercise idle recovery and long-term drift.
Additional tests check cycle splitting, fractional/reversed bounds, live range
changes, legacy settings migration, persisted choices, and worker snapshots.

To regenerate `src/engine/vapeReferenceTodoAi.csv`, compile
`tests/ReferenceTodoAi.java` with the two timing Java source files listed above
into a temporary class directory, then run `java -cp <class-dir> ReferenceTodoAi`.
The CSV stores FNV-1a checksums over little-endian signed 64-bit intervals. No
Java runtime is needed for the Rust tests. These tests validate policy and config
behavior, not live Windows input timing equivalence.
