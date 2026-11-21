# FoamCraft

FoamCraft is a free, open-source desktop app that makes CFD simulation with OpenFOAM approachable:
load a 3D model, click where the flow goes in and out or place the model in a virtual wind tunnel,
run the simulation, then explore, animate and export the results.

**Status:** early development. The app starts with a 3D view; simulation features follow the
work plan in [Roadmap/](Roadmap/README.md). Project documentation: [docs/](docs/project-brief.md).

## Development

```bash
npm ci                 # JS tooling and frontend dependencies
npm run app:dev        # start the desktop app in development mode
cargo test --workspace # Rust tests
npm test               # frontend tests
```

## Planned features

- Import of STL, OBJ, 3MF, STEP and IGES geometry
- Virtual wind tunnel (external flow) and internal flow with click-to-assign inlets and outlets
- Automatic meshing, solver setup and convergence detection
- GPU-rendered 3D results with animated particles, slices and iso-surfaces
- Image, video and report export (PDF, HTML)
- MCP server so AI agents can set up, run and evaluate simulations
- Runs locally on Windows, Linux and macOS; OpenFOAM runs in a managed container

## License

FoamCraft is licensed under the [GNU General Public License v3.0 or later](LICENSE).
Dependency and asset rules are in [docs/licensing.md](docs/licensing.md).

## Trademark notice

FoamCraft is an independent project. This offering is not approved or endorsed by OpenCFD Limited,
producer and distributor of the OpenFOAM software via www.openfoam.com, and owner of the OPENFOAM®
and OpenCFD® trade marks.

## Disclaimer

Simulation results are estimates. They are not certified and come without any warranty; do not use
them as the sole basis for safety-relevant decisions.
