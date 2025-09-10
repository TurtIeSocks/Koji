#include "distance_matrix.h"
#include <thread>
#include <cmath>
#include <algorithm>

namespace
{
  constexpr double kPi = 3.141592653589793238462643383279502884;
  constexpr double kDeg2Rad = kPi / 180.0;
  constexpr double kEarthRadiusMeters = 6372800.0;

  struct NodeRad
  {
    double lat;
    double lon;
    double cos_lat;
  };

  inline double sqr(double x) noexcept { return x * x; }

  inline double haversine_rad(double lat1, double lon1,
                              double lat2, double lon2,
                              double cos_lat1, double cos_lat2) noexcept
  {
    const double dlat = lat2 - lat1;
    const double dlon = lon2 - lon1;
    const double sdlat = std::sin(0.5 * dlat);
    const double sdlon = std::sin(0.5 * dlon);
    const double a = sqr(sdlat) + sqr(sdlon) * (cos_lat1 * cos_lat2);
    const double c = 2.0 * std::asin(std::min(1.0, std::sqrt(a)));
    return kEarthRadiusMeters * c;
  }

  std::vector<NodeRad> precompute_nodes(const RawInput &locs)
  {
    std::vector<NodeRad> out;
    out.reserve(locs.size());
    for (const auto &p : locs)
    {
      const double lat = p.first * kDeg2Rad;
      const double lon = p.second * kDeg2Rad;
      out.push_back(NodeRad{lat, lon, std::cos(lat)});
    }
    return out;
  }

  void compute_block(const std::vector<NodeRad> &nodes,
                     DistanceMatrix &dist,
                     size_t row_begin, size_t row_end)
  {
    const size_t n = nodes.size();
    for (size_t i = row_begin; i < row_end; ++i)
    {
      dist.at(i, i) = 0;
      const auto &ni = nodes[i];
      for (size_t j = i + 1; j < n; ++j)
      {
        const auto &nj = nodes[j];
        const double meters = haversine_rad(
            ni.lat, ni.lon, nj.lat, nj.lon,
            ni.cos_lat, nj.cos_lat);
        const int64_t m = static_cast<int64_t>(meters + 0.5);
        dist.at(i, j) = m;
        dist.at(j, i) = m;
      }
    }
  }
}

DistanceMatrix distanceMatrix(const RawInput &locations)
{
  const size_t n = locations.size();
  DistanceMatrix dm(n);
  if (n <= 1)
  {
    if (n == 1)
      dm.at(0, 0) = 0;
    return dm;
  }

  const auto nodes = precompute_nodes(locations);

  unsigned hw = std::thread::hardware_concurrency();
  size_t num_threads = std::max<size_t>(1, hw ? hw : 1);
  num_threads = std::min(num_threads, n);

  std::vector<std::thread> pool;
  pool.reserve(num_threads);

  const size_t base = n / num_threads;
  size_t extra = n % num_threads;
  size_t row = 0;
  for (size_t t = 0; t < num_threads; ++t)
  {
    const size_t take = base + (extra ? 1 : 0);
    if (extra)
      --extra;
    const size_t begin = row;
    const size_t end = begin + take;
    row = end;
    pool.emplace_back([&nodes, &dm, begin, end]()
                      { compute_block(nodes, dm, begin, end); });
  }
  for (auto &th : pool)
    th.join();
  return dm;
}
