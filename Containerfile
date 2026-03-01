FROM docker.io/library/rust:1-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    libasound2-dev libespeak-ng-dev libspeechd-dev \
    pkg-config cmake libvulkan-dev glslc clang \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
