include(${CMAKE_CURRENT_LIST_DIR}/get-corrosion.cmake)

add_subdirectory(glue)

# ---- Rust extensions (each one uses frost_extension) ----
# frost_extension(example_rs)
