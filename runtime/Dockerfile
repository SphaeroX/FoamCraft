# SPDX-License-Identifier: GPL-3.0-or-later
#
# FoamCraft runtime image: OpenFOAM (OpenCFD release, pinned) plus the FoamCraft agent.
# Build from the repository root:
#   docker build -f runtime/Dockerfile -t foamcraft-runtime:<version> .
# (`foamcraft runtime build` does exactly this.)
#
# OpenFOAM is GPL-3.0-or-later. The labels below name the exact upstream release so the
# corresponding source can be obtained (see docs/licensing.md).

ARG OPENFOAM_API=2606
ARG OPENFOAM_BASE=opencfd/openfoam-run:2606@sha256:4229997e74defb81548222d511b8e3b95b98305e5df41b8e88b031813fe47eeb

# --- Stage 1: Linux build of the agent ------------------------------------------------------
FROM rust:1.99-bookworm AS agent
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY app/src-tauri/Cargo.toml ./app/src-tauri/Cargo.toml
# The Tauri crate is a workspace member; a stub source keeps the manifest valid without
# compiling the desktop app.
RUN mkdir -p app/src-tauri/src && echo 'fn main() {}' > app/src-tauri/src/main.rs \
    && echo 'fn main() {}' > app/src-tauri/build.rs
RUN cargo build --release --locked -p foamcraft-agent

# --- Stage 2: runtime ---------------------------------------------------------------------
FROM ${OPENFOAM_BASE}
ARG OPENFOAM_API
LABEL org.opencontainers.image.title="FoamCraft runtime" \
      org.opencontainers.image.licenses="GPL-3.0-or-later" \
      org.opencontainers.image.source="https://github.com/SphaeroX/FoamCraft" \
      org.foamcraft.openfoam.version="v${OPENFOAM_API}" \
      org.foamcraft.openfoam.base-image="opencfd/openfoam-run:2606@sha256:4229997e74defb81548222d511b8e3b95b98305e5df41b8e88b031813fe47eeb" \
      org.foamcraft.openfoam.source="https://develop.openfoam.com/Development/openfoam/-/tree/OpenFOAM-v${OPENFOAM_API}" \
      org.foamcraft.openfoam.build="_481094f-20260618" \
      org.foamcraft.notice="This offering is not approved or endorsed by OpenCFD Limited, producer and distributor of the OpenFOAM software via www.openfoam.com, and owner of the OPENFOAM and OpenCFD trade marks."
# Gmsh (GPL-2.0-or-later) with Open CASCADE (LGPL-2.1) converts STEP and IGES files into
# triangle meshes that keep the CAD faces (Task_07). Sources: the Ubuntu packages.
RUN apt-get update \
    && apt-get install -y --no-install-recommends gmsh \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*
COPY --from=agent /src/target/release/foamcraft-agent /usr/local/bin/foamcraft-agent
RUN mkdir -p /work && chmod 0777 /work
WORKDIR /work
ENTRYPOINT ["/usr/local/bin/foamcraft-agent"]
CMD ["idle"]
