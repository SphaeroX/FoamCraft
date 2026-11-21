# MCP server: FoamCraft for AI agents

FoamCraft has a Model Context Protocol (MCP) server. An AI agent (for example Claude Code) uses it
to set up a project, run a simulation, read the key figures and get images of the 3D view.

## Start

```bash
foamcraft mcp --root <folder> [--root <another folder>] [--app]
```

- **Transport:** stdio (one JSON-RPC message per line). No HTTP.
- **`--root`** (required, repeatable): the only folders the agent may use. Every path argument of
  every tool (`path`, `folder`, `file`, `source`, keys ending in `_path` or `_dir`) must be
  absolute and lie inside a root; `..` is resolved before the check. Anything else is refused with
  `mcp.path_outside_roots`. `project_new` and `samples.create` without a path create a free folder
  in the first root (not in the user's default projects folder).
- **Without `--app`:** the server has its own session, renders offscreen and needs no window.
- **With `--app`:** the commands run in the running FoamCraft app, so the user sees every step;
  the server stops with `bridge.no_app` when the app is not running.

## Configuration for an MCP client

The app shows this entry ready to copy under **Agents** in the toolbar. By hand:

```json
{
  "mcpServers": {
    "foamcraft": {
      "command": "C:\\Program Files\\FoamCraft\\foamcraft.exe",
      "args": ["mcp", "--root", "C:\\Users\\me\\CFD", "--app"]
    }
  }
}
```

Claude Code: put the entry into `.mcp.json` of a project, or run
`claude mcp add foamcraft -- "C:\Program Files\FoamCraft\foamcraft.exe" mcp --root C:\Users\me\CFD`.
In development the program is `target/release/foamcraft.exe` (`cargo build --release -p
foamcraft-cli`).

## Tools

| Area      | Tools                                                                                                                                                                                                             |
| --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Project   | `project_new`, `project_open`, `project_info`                                                                                                                                                                     |
| Geometry  | `geometry_import`, `geometry_info`, `geometry_list`, `geometry_set_transform`, `geometry_pivot`, `geometry_move_by`, `geometry_rotate_about`, `surface_select_at`, `geometry_cap_opening`                         |
| Scenario  | `scenario_list`, `scenario_add`, `scenario_activate`, `scenario_set_wind_tunnel`, `scenario_set_wind_direction`, `scenario_set_jets`, `scenario_set_internal_flow`, `scenario_set_boundary_role`, `scenario_info` |
| Run       | `run_start` (answers at once with `job_id`; meshes first when needed), `run_wait` (time limit up to 3600 s), `run_status`, `run_cancel`, `run_log` (meshing and solver)                                           |
| Results   | `results_summary`, `results_probe`, `results_compare`, `results_compare_many`, `runs_list`                                                                                                                        |
| View      | `view_set_surface_field`, `view_add_slice`, `view_apply_preset`, `view_set_surface_flow`, `view_render` (PNG image in the answer, optional standard `view` or `ride_t`)                                           |
| Particles | `view_set_particles` (styles comets, smoke, streamlines, trails, ribbons, tubes; heavy particles with `inertia`), `particles_materials`, `results_impact`, `results_impact_status`                                |
| Ride      | `ride_suggest`, `ride_start` (suggestion id or world point `from`; answers with the readings and events), `ride_telemetry`, `ride_stop`                                                                           |
| Export    | `export_image` (`exposure_s`: long exposure of the particles), `export_video` (answers at once with `export_id`), `export_status`, `export_cancel`                                                                |
| Report    | `export_report` (PDF and/or HTML, `language` en or de, `compare_with` for two runs)                                                                                                                               |
| Generic   | `commands_list`, `command_describe`, `command_run` (every command of the command API)                                                                                                                             |

Answers are compact JSON in SI units. `results_summary` carries the English explanation of every
figure as `text`. Errors are tool errors with `{code, message, fix, detail}`; the codes are those
of the message catalog.

The server instructions describe the usual order; the resource `foamcraft://guide` explains how to
set up a simulation and how to check that its results are plausible.

## Bridge to the app

The app listens on a loopback TCP port (127.0.0.1, chosen by the system) and writes port and a
random token to `<data folder>/app-bridge.json` (`%LOCALAPPDATA%\FoamCraft` on Windows). A request
without the token is refused (`bridge.refused`). Commands from the bridge run in the app's own
session, so the interface updates through the usual `project.changed` events. See
`docs/architecture.md`, section "MCP server".
