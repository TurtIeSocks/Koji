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
typedef std::vector<int> RawOutput;

namespace operations_research
{
  const double R = 6372.8; // km

  struct DataModel
  {
    DistanceMatrix distance_matrix;
    const int num_vehicles = 1;
    const RoutingIndexManager::NodeIndex depot{0};
  };

  //! @brief Computes the distance between two nodes using the Haversine formula.
  //! @param[in] lat1 Latitude of the first node.
  //! @param[in] lon1 Longitude of the first node.
  //! @param[in] lat2 Latitude of the second node.
  //! @param[in] lon2 Longitude of the second node.
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

  //! @brief Computes the distance matrix between all nodes.
  //! @param[in] locations The locations of the nodes.
  //! @param[out] distances The distance matrix between all nodes.
  //! @param[in] start The index of the first node to compute.
  //! @param[in] end The index of the last node to compute.
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

  //! @brief Computes the distance matrix between all nodes.
  //! @param[in] locations The [Lat, Lng] pairs.
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

  //! @brief Returns the routes of the solution.
  //! @param[in] manager The manager of the routing problem.
  //! @param[in] routing The routing model.
  //! @param[in] solution The solution of the routing problem.
  RawOutput GetRoutes(const RoutingIndexManager &manager, const RoutingModel &routing, const Assignment &solution)
  {
    RawOutput routes(manager.num_vehicles());
    for (int vehicle_id = 0; vehicle_id < manager.num_vehicles(); ++vehicle_id)
    {
      int64_t index = routing.Start(vehicle_id);
      routes.push_back(manager.IndexToNode(index).value());
      while (!routing.IsEnd(index))
      {
        index = solution.Value(routing.NextVar(index));
        routes.push_back(manager.IndexToNode(index).value());
      }
    }
    return routes;
  }

  //! @brief Solves the TSP problem.
  //! @param[in] locations The [Lat, Lng] pairs.
  RawOutput Tsp(RawInput locations)
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

    if (locations.size() > 1000)
    {
      searchParameters.set_local_search_metaheuristic(
          LocalSearchMetaheuristic::GUIDED_LOCAL_SEARCH);
      int64_t time = std::max(std::min(pow(locations.size() / 1000, 2.75), 3600.0), 3.0);
      searchParameters.mutable_time_limit()->set_seconds(time);
      // LOG(INFO) << "Time limit: " << time;
    }
    // searchParameters.set_log_search(true);

    const Assignment *solution = routing.SolveWithParameters(searchParameters);

    return GetRoutes(manager, routing, *solution);
  }

}

std::vector<std::string> split(const std::string &s, char delimiter)
{
  std::vector<std::string> tokens;
  std::string token;
  std::istringstream tokenStream(s);
  while (std::getline(tokenStream, token, delimiter))
  {
    tokens.push_back(token);
  }
  return tokens;
}

int main(int argc, char *argv[])
{
  std::map<std::string, std::string> args;
  RawInput points;
  std::vector<std::string> stringPoints;

  for (int i = 1; i < argc; ++i)
  {
    std::string arg = argv[i];
    if (arg.find("--") == 0)
    {
      std::string key = arg.substr(2);
      if (key == "input")
      {
        std::string pointsStr = argv[++i];
        std::vector<std::string> pointStrings = split(pointsStr, ' ');
        for (const auto &pointStr : pointStrings)
        {
          auto coordinates = split(pointStr, ',');
          if (coordinates.size() == 2)
          {
            double lat = std::stod(coordinates[0]);
            double lng = std::stod(coordinates[1]);
            points.push_back({lat, lng});
            stringPoints.push_back(pointStr);
          }
        }
      }
      else if (i + 1 < argc)
      {
        args[key] = argv[++i];
      }
    }
  }

  RawOutput routes = operations_research::Tsp(points);

  for (auto point : routes)
  {
    std::cout << stringPoints[point] << " ";
  }

  return EXIT_SUCCESS;
}