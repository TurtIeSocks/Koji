#include "koji/src/clustering/main.h"
#include "koji/src/clustering/udc.h"
#include <algorithm>
#include <cassert>
#include <iostream>
#include <iterator>
#include <vector>

rust::Vec<CppPoint> concat(rust::Vec<CppPoint> r)
{
  // Test Rust input
  std::cout << "C++ Print:" << std::endl;
  for (auto coord : r)
  {
    std::cout << coord.x << ", " << coord.y << std::endl;
  }

  // Copy into Cpp Vector
  std::vector<Point> P;
  for (auto coord : r)
  {
    P.push_back(Point(coord.x, coord.y));
  }

  // Run the clustering
  std::list<Point> C;
  FASTCOVER_PP ob(P, C);

  std::cout << "Time: " << ob.execute() << " seconds" << std::endl;

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
