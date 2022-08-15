#pragma once
#include "koji/src/clustering/bridge.rs.h"
#include "rust/cxx.h"

rust::Vec<CppPoint> clustering(rust::Vec<CppPoint> r);
