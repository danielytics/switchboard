# Switchboard

Switchboard is a tool to turn any USB keyboard into a hotkey panel (making a normal USB keyboard act kind of like a Stream Deck).

## Features

* Bind keys to actions
* Execute programs/scripts as actions
* Make HTTP requests as actions
* Memory cells (increment, decrement, set, reset), changes can trigger actions, current value can be used as template variables
* Timers (start, cancel, restart), trigger action on timeout
* Multiple layers (shift, toggle)

## Running

* `switchboard` - Run switchboard
* `switchboard device add` - Add a new device. Will prompt for a key press on the desired device.
* `switchboard device list` - List configured devices.
* `switchboard device remove <name>` - Remove a device.

Device changes don't take effect until next run.

Switchboard will run in the background and will automatically capture (exclusive access) key press events from the configured devices.

Switchboard will create a tray icon from which its pause state can be toggled, and the config UI can be opened.

The config UI allows you to view and manage (add, remove) devices, configure layers, and configure keybindings. That is, everything that the config files can do, the UI can do.

## Configuration

Config is loaded from `~/.config/switchboard/`, which has the following structure:

```
├── config.toml
├── devices.toml
└── profiles
    ├── default.toml
    └── <...>.toml
```

This file structure is created on first run, if it doesn't exist.

### Device selection

This is stored in `~/.config/switchboard/devices.toml`

```toml
[[devices]]
  vid = ...
  pid = ...
  iface = ...
  active = true # Optional, defaults to true. Set to false to temporarily disable
  profile = "name" # Specify which profile to load for this device
```

Each device is bound to a "profile", which is a set of keybindings that are applied to this device.

### Key bindings

All keybindings are defined in a proifle in `~/.config/switchbaord/profiles/<profile-name>.toml`.

```toml
[[bind]]
  layer = "default" # "Optional layer
  key = "a" # Scancode name string or numeric value
  [bind.action]
    ...
```

You may bind the same key multiple times and all bound actions on the currently selected layer will trigger.

Multiple devices are supported by setting the `device` key. If not set, `default` is used.

Keys also have an optional secondary actions:

```toml
[[bind]]
  key = "a" # Scancode name string or numeric value
  [bind.action-up]
    ...
```

This is triggered when the key is released.

### Execute Programs or Scripts

```toml
[bind.action]
  type = "exec"
  path = "/path/to/program" # The program or script to run
  args = ["--arg", "value", "{{template_var}}"] # Optional arguments
  [bind.action.env] # Optional environment variables
    SOME_ENV_VAR = "value"
    ANOTHER_VAR = true # Setting av variable to true loads it from .env or parent environment, if available
```

### HTTP Requests

```toml
[bind.action]
  type = "http"
  method = "POST" # "GET", "POST", "PUT", "DELETE", etc.
  url = "http://example.com/api/{{template_var}}"
  query = {a = 1} # Optional query string arguments
  body = {} # Optional body, table will be converted to JSON, string will be sent as-is
```

### Memory Cells

First, you need to create one, with a top-level config:

```toml
[cells.counter] # "counter" is this cells name, you can set any name
  min = 1 # Minimum value
  max = 10 # Maximum value
  default = 5 # Value to start at and reset to, defaults to `min` if omitted
  wrap = true # Wether to wrap (true) or clamp (false). Defaults to `false` if omitted
  settle_ms = 100 # Optional time between multiple changes before triggering on-change actions
```

Then you can manipulate the cell with actions:

```toml
[bind.action]
  type = "cell"
  cell = "counter"
  command = "increment" # "increment", "decrement", "reset", "set"
  value = 3 # Only if command is "set"
```

You can also bind cell changes to actions:

```toml
[[bind]]
  key = 0
  binding = {type="cell", name="counter", filter=5} # Filter can be a numeric value (only perform action when cell becomes this value), or a command (send the action when this command is run), omit filter to send on all changes
  [bind.action]
    ...
```

Cells can also map their values to strings (for tepmlates; filter still operates on raw numbers):

```toml
[cells.counter]
  min = 1
  max = 5
  [cells.mapping]
    1 = "one"
    2 = "two"
    3 = "three"
    4 = "four"
    5 = "five"
```

Cells are made available in templates variables:

* `{{cells.<name>}}` - the current (mapped, if available) value
* `{{cells.<name>.raw}}` - the current numeric value (never the mapped value)
* `{{cells.<name>.pct}}` - 0..100 percentage value `((current - min) / (max - min)) * 100`

### Timers

First, you must create a new timer:

```toml
[timer.delay] # "delay" is this timers name, you can set any name
  timeout_ms = 100 # Milliseconds from start to timeout
  repeat = true # Optionally repeat at the timeout_ms interval until stopped, default to false
```

Then you can control the timer with actions:

```toml
[bind.action]
  type = "timer"
  cell = "delay"
  command = "start" # "start", "stop", "restart" (restart resets the timer and starts it again)
  value = 3 # Only if command is "set"
```

Starting an already started timer is a no-op. Only one instance of a timer can be active at once.

Binding a timer to an action:

```toml
[[bind]]
  key = 0
  binding = {type="timer", name="delay"}
  [bind.action]
    ...
```

### Layers

Layers act as a filter for bindings. Only bindings on the active layer(s) will perform their actions when triggered. Layers can be part of an exclusive group. Only one layer of each exclusive group can be active at once (the most recently activated layer).

An implicit layer `"default"` is automatically created and activated. All bindings without a set layer will be placed in this layer. You CAN manually deactivate this layer, although you shouldn't.

Creating layers:

```toml
[[layers]]
  name = "layer-1"
  active = true # Whether active or inactive by default, defaults to false.
  exclusive-group = "group1" # Only the last activated layer in each exclusive group is active, all others are inactivated
[[layers]]
  name = "layer-b"
  exclusive-group = "group1" # Only the last activated layer in each exclusive group is active, all others are inactivated
```

All layers (regardless of group) must have unique names. A layer cannot be in multiple groups at once.

Using a layer:

```toml
[[bind]]
  layer = "layer-b" # This binding will only trigger if "layer-b" is active
  ...
```

Layers can be controlled through actions

```toml
[bind.action]
  type = "layer"
  layer = "layer-b"
  command = "active" # "active", "inactive"
```

You can create shifted layers:

```toml
[[bind]]
  key = "a" # Turn `a` into the layer shift key
  [bind.action]
    type = "layer"
    layer = "layer-b"
    command = "active"
  [bind.action-up]
    type = "layer"
    layer = "layer-b"
    command = "inactive"
```

Its also possible to manipulate exclusive groups:

```toml
[bind.action]
  type = "layer-group"
  group = "exclusive-group-1"
  command = "clear" # "clear", "next", "previous"
```

* `clear` will deactivate all layers in the group (ie, if a layer is active, deactivate it, regardless of which layer it is).
* `next` will activate the next layer in the group (next is determined by the order that layers are defined in the `[[layers]]` array)
* `prev` will activate the previous layer in the group (previous is determined by the order that layers are defined in the `[[layers]]` array)

## Other Tools

* [ch57x-keyboard-tool](https://github.com/kriomant/ch57x-keyboard-tool) was born from a similar desire: getting keypads to work without the manufacturers Windows-based config tools.

This utility targets the same devices, but works very differently: it configures the devices themselves to send your key bindings; its like an expanded programmable keyboard. Switchboard runs entirely off-device and routes the devices default key bindings to any action you choose.
