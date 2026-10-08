// Debug traces for the C++ side: compiled in only with `--features trace` (build.rs defines MARKITE_TRACE);
// absent from release builds.
#pragma once
#include <cstdio>

#ifdef MARKITE_TRACE
#define TRACE(...) do { std::fprintf(stderr, "[trace native] " __VA_ARGS__); std::fputc('\n', stderr); } while (0)
#else
#define TRACE(...) ((void)0)
#endif
