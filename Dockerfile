FROM gcc:15 AS builder

ARG BUILD_TYPE=Release
ARG WITH_HTTP=No

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        cmake \
        mold \
        libboost-json-dev \
        libboost-regex-dev \
    && if [ "${WITH_HTTP}" = "Yes" ]; then \
        apt-get install -y --no-install-recommends \
            libboost-url-dev \
            libssl-dev; \
    fi \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . /src

RUN cmake -S . -B /build -DCMAKE_BUILD_TYPE=${BUILD_TYPE} -DBUILD_TESTS=No -DWITH_HTTP=${WITH_HTTP} \
    && cmake --build /build -j8 \
    && if [ "${BUILD_TYPE}" = "Release" ]; then strip /build/frost; fi

FROM debian:trixie-slim AS runtime

ARG WITH_HTTP=No

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libboost-container1.83.0 \
        libboost-json1.83.0 \
        libboost-regex1.83.0 \
        libgcc-s1 \
        libstdc++6 \
        libzstd1 \
        zlib1g \
    && if [ "${WITH_HTTP}" = "Yes" ]; then \
        apt-get install -y --no-install-recommends \
            libboost-url1.83.0 \
            libssl3t64; \
    fi \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/frost /usr/local/bin/frost

WORKDIR /work
ENTRYPOINT ["/usr/local/bin/frost"]
