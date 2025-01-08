# phomemo-d30

Library & utilities for controlling the [Phomemo D30](https://phomemo.com/products/d30-label-maker) label maker, using a reverse engineered protocol.

This library contains components heavily based on code available in the [polskafan phomemo_d30](https://github.com/polskafan/phomemo_d30) repo,
but takes no code directly from said library. That library in turn is based heavily on the work of others,
including [viver](https://github.com/vivier/phomemo-tools) and [theacodes](https://github.com/theacodes/phomemo_m02s).

The gist of it is that there are several magic sequences sent to the appliance by their 'Print Master' Android app. These were sniffed,
and now can be blindly transmitted by a number of scripts and utilities available on Github. This is one such utility.

---

This is a label preview tool for the phomemo-d30 suite, intended as a workaround for some (questionably designed) OSes not allowing secondary threads to draw GUIs. For usage instructions, see the [git repo](https://github.com/crabdancing/phomemo-d30).