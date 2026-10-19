# Quickstart: validating Settings accordion groups

## Part A: automated

```sh
mise run test-core                      # unaffected, quick
cargo test -p micold-client accordion   # via: mise run test
mise run gate
```

Expected: `accordion_group` tests, `daemons_feature` section tests, `showcase_completeness` and
`daemons_section_geometry` pass.

## Part B: visual pass (needs eyes at a display; see the `visual-pass` skill)

1. Launch the component showcase, open `AccordionSection`. In the group, open section 2: section 1
   closes while 2 opens, without the page jumping. Press 2 again: none open. In the independent
   set, open two sections: both stay open.
2. Launch the client, Settings > Daemons, add a daemon of each kind (Container, Kubernetes, Remote,
   SSH): the first section is open; open another: the first closes; press the open one: all
   closed. Use Space and Enter on a focused header: the same.
3. Leave a required field empty in a closed section: its title shows the marker and the section is
   open; open another section: both stay open; fill the field: the first closes, the other stays.
4. Light and dark theme: no overlap or flicker while one closes and the other opens.
5. Switch *Runs in*: only the new kind's first section is open.
