#include "koji/src/cpp/clustering/main.h"
#include "koji/src/cpp/clustering/udc.h"
#include <algorithm>
#include <cassert>
#include <iostream>
#include <iterator>
#include <vector>

rust::Vec<CppPoint> clustering(rust::Vec<CppPoint> r)
{
  // Copy into Cpp Vector
  std::vector<Point> P;
  for (auto coord : r)
  {
    P.push_back(Point(coord.x, coord.y));
  }

  // Run the clustering
  std::list<Point> C;
  FASTCOVER_PP ob(P, C);

  std::cout << "[CLUSTER] Time: " << ob.execute() << " seconds" << std::endl;

  // Copy back into Rust Vector
  rust::Vec<CppPoint> result;
  for (auto coord : C)
  {
    CppPoint po;
    po.x = coord.x();
    po.y = coord.y();
    result.push_back(po);
  }
  return result;
}
