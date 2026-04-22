FROM gcc:15 AS builder

ARG BUILD_TYPE=Release

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        cmake \
        mold \
        libboost-json-dev \
        libboost-regex-dev \
        libboost-url-dev \
        libboost-filesystem-dev \
        libboost-dev \
        libssl-dev \
        zlib1g-dev \
        libbz2-dev \
        liblzma-dev \
        libbrotli-dev \
        liblz4-dev \
        libsnappy-dev \
        libzstd-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . /src

RUN cmake -S . -B /build -DCMAKE_BUILD_TYPE=${BUILD_TYPE} -DBUILD_TESTS=No \
    && cmake --build /build -j$(nproc) \
    && if [ "${BUILD_TYPE}" = "Release" ]; then strip /build/frost; fi

FROM gcc:15 AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libboost-container1.83.0 \
        libboost-json1.83.0 \
        libboost-regex1.83.0 \
        libboost-url1.83.0 \
        libboost-filesystem1.83.0 \
        libboost-atomic1.83.0 \
        libssl3t64 \
        zlib1g \
        libbz2-1.0 \
        liblzma5 \
        libbrotli1 \
        liblz4-1 \
        libsnappy1v5 \
        libzstd1 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/frost /usr/local/bin/frost

WORKDIR /work
ENTRYPOINT ["/usr/local/bin/frost"]
