file(GLOB _FOUND "${CARGO_BUILD_DIR}/build/frost-example-*/out/example.cpp")
list(GET _FOUND 0 _SRC)
file(COPY_FILE ${_SRC} ${DEST})
