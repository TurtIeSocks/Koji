#ifndef DISTANCEMATRIX_H
#define DISTANCEMATRIX_H

#include <vector>
#include <cstdint>
#include <utility> // for std::pair
#include <cstddef>

using RawInput = std::vector<std::pair<double, double>>;

// Flat, row-major (n x n) distance matrix in meters, int64_t
struct DistanceMatrix
{
  std::vector<int64_t> data;
  size_t n = 0;

  DistanceMatrix() = default;
  explicit DistanceMatrix(size_t n_) : data(n_ * n_), n(n_) {}

  inline int64_t &at(size_t i, size_t j) { return data[i * n + j]; }
  inline int64_t at(size_t i, size_t j) const { return data[i * n + j]; }
};

// Compute full symmetric distance matrix
DistanceMatrix distanceMatrix(const RawInput &locations);

#endif // DISTANCEMATRIX_H
