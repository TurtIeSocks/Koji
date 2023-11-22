// This file must be compiled in the same directory as OR-Tools

#include <cmath>
#include <cstdint>
#include <sstream>
#include <vector>
#include <iostream>
#include <string>
#include <thread>

#include "ortools/constraint_solver/routing.h"
#include "ortools/constraint_solver/routing_enums.pb.h"
#include "ortools/constraint_solver/routing_index_manager.h"
#include "ortools/constraint_solver/routing_parameters.h"

typedef std::vector<std::vector<int64_t>> DistanceMatrix;
typedef std::vector<std::vector<double>> RawInput;

namespace operations_research
{
  const double R = 6372.8; // km

  struct DataModel
  {
    DistanceMatrix distance_matrix;
    const int num_vehicles = 1;
    const RoutingIndexManager::NodeIndex depot{0};
  };

  double haversine(double lat1, double lon1, double lat2, double lon2)
  {

    double dLat = (lat2 - lat1) * M_PI / 180.0;
    double dLon = (lon2 - lon1) * M_PI / 180.0;
    lat1 = lat1 * M_PI / 180.0;
    lat2 = lat2 * M_PI / 180.0;

    double a = pow(sin(dLat / 2), 2) + pow(sin(dLon / 2), 2) * cos(lat1) * cos(lat2);
    double c = 2 * asin(sqrt(a));
    return R * c * 1000; // to reduce rounding issues
  }

  void computeDistances(const RawInput &locations, DistanceMatrix &distances, int start, int end)
  {
    for (int fromNode = start; fromNode < end; ++fromNode)
    {
      for (int toNode = 0; toNode < locations.size(); ++toNode)
      {
        if (fromNode != toNode)
        {
          distances[fromNode][toNode] = static_cast<int64_t>(
              haversine(locations[toNode][0], locations[toNode][1],
                        locations[fromNode][0], locations[fromNode][1]));
        }
      }
    }
  }

  DistanceMatrix distanceMatrix(const RawInput &locations)
  {
    auto start = std::chrono::high_resolution_clock::now();

    int numThreads = std::thread::hardware_concurrency();

    std::vector<std::thread> threads(numThreads);
    DistanceMatrix distances = DistanceMatrix(locations.size(), std::vector<int64_t>(locations.size(), int64_t{0}));

    int chunkSize = locations.size() / numThreads;
    for (int i = 0; i < numThreads; ++i)
    {
      int start = i * chunkSize;
      int end = (i == numThreads - 1) ? locations.size() : start + chunkSize;
      threads[i] = std::thread(computeDistances, std::ref(locations), std::ref(distances), start, end);
    }

    for (auto &thread : threads)
    {
      thread.join();
    }
    return distances;
  }

  RawInput GetRoutes(const RoutingIndexManager &manager, const RoutingModel &routing, const Assignment &solution)
  {
    RawInput routes(manager.num_vehicles());
    for (double vehicle_id = 0; vehicle_id < manager.num_vehicles(); ++vehicle_id)
    {
      int64_t index = routing.Start(vehicle_id);
      routes[vehicle_id].push_back(manager.IndexToNode(index).value());
      while (!routing.IsEnd(index))
      {
        index = solution.Value(routing.NextVar(index));
        routes[vehicle_id].push_back(manager.IndexToNode(index).value());
      }
    }
    return routes;
  }

  RawInput Tsp(RawInput locations)
  {
    DataModel data;
    data.distance_matrix = distanceMatrix(locations);
    RoutingIndexManager manager(data.distance_matrix.size(), data.num_vehicles,
                                data.depot);
    RoutingModel routing(manager);

    const int transit_callback_index = routing.RegisterTransitCallback(
        [&data, &manager](int64_t from_index, int64_t to_index) -> int64_t
        {
          auto from_node = manager.IndexToNode(from_index).value();
          auto to_node = manager.IndexToNode(to_index).value();
          return data.distance_matrix[from_node][to_node];
        });

    routing.SetArcCostEvaluatorOfAllVehicles(transit_callback_index);
    RoutingSearchParameters searchParameters = DefaultRoutingSearchParameters();
    searchParameters.set_first_solution_strategy(
        FirstSolutionStrategy::PATH_CHEAPEST_ARC);

    const Assignment *solution = routing.SolveWithParameters(searchParameters);

    return GetRoutes(manager, routing, *solution);
  }

}

int main()
{
  RawInput distance_matrix;
  std::vector<double> row;

  std::string line;
  while (std::getline(std::cin, line, ',') && !line.empty())
  {
    if (line == " ")
    {
      distance_matrix.push_back(row);
      row.clear();
      continue;
    }
    double value = std::stod(line);
    row.push_back(value);
  }

  RawInput routes = operations_research::Tsp(distance_matrix);
  for (auto route : routes)
  {
    for (auto node : route)
    {
      std::cout << node << ",";
    }
    std::cout << std::endl;
  }

  return EXIT_SUCCESS;
}