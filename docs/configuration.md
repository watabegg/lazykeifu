# Configuration

lazykeifu can be configured via `~/.config/lazykeifu/config.toml`. All settings are optional.

## Auto-refresh

By default, lazykeifu automatically refreshes the commit graph every 10 seconds and fetches from origin every 60 seconds.

```toml
[refresh]
# Enable auto-refresh for local state (default: true)
auto_refresh = true

# Interval in seconds for local refresh (default: 10, minimum: 1)
refresh_interval = 10

# Enable auto-fetch from origin (default: true)
auto_fetch = true

# Interval in seconds for remote fetch (default: 60, minimum: 10)
fetch_interval = 60
```

## Graph display

By default, lazykeifu shows remote branches and commits that are reachable only from
remote branches. You can hide them by default:

```toml
[graph]
# Show remote branches by default (default: true)
show_remote_branches = false
```

Press `o` in the TUI to toggle remote branches for the current session.

By default, lazykeifu shows tag labels on commits. You can hide them by default:

```toml
[graph]
# Show tag labels by default (default: true)
show_tags = false
```

Press `t` in the TUI to toggle tags for the current session.

## File diff view

By default, opening a file diff takes over the whole screen. To draw it in the
graph pane instead, keeping commit detail and the file list visible:

```toml
[ui]
# "fullscreen" (default) or "replace"
file_view = "replace"
```

The config can be overridden for a single run with `lazykeifu --file-view replace`.

### Options

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `auto_refresh` | bool | `true` | Enable auto-refresh for local state (commits, branches, working tree) |
| `refresh_interval` | integer | `10` | Interval in seconds for local refresh (minimum: 1) |
| `auto_fetch` | bool | `true` | Enable auto-fetch from origin |
| `fetch_interval` | integer | `60` | Interval in seconds for remote fetch (minimum: 10) |
| `graph.show_remote_branches` | bool | `true` | Show remote branches and commits reachable only from remote branches |
| `graph.show_tags` | bool | `true` | Show tag labels on commits |
| `ui.file_view` | string | `"fullscreen"` | Draw the file diff fullscreen, or in place of the graph pane (`"replace"`) |

### Disabling auto-refresh

To disable automatic updates entirely:

```toml
[refresh]
auto_refresh = false
auto_fetch = false
```

You can still manually refresh with `R` and fetch with `f`.
