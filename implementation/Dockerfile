FROM gcc:15

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        cmake \
        libboost-json-dev \
        libboost-regex-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . /app

RUN cmake -S . -B build-directory -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTS=No \
    && cmake --build build-directory -j8

WORKDIR /work
ENTRYPOINT ["/app/build-directory/frost"]
