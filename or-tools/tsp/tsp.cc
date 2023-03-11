// This file must be compiled in the same directory as OR-Tools

#include <cmath>
#include <cstdint>
#include <sstream>
#include <vector>
#include <iostream>
#include <string>

#include "ortools/constraint_solver/routing.h"
#include "ortools/constraint_solver/routing_enums.pb.h"
#include "ortools/constraint_solver/routing_index_manager.h"
#include "ortools/constraint_solver/routing_parameters.h"

namespace operations_research
{
  struct DataModel
  {
    std::vector<std::vector<double>> distance_matrix;
    const int num_vehicles = 1;
    const RoutingIndexManager::NodeIndex depot{0};
  };

  std::vector<std::vector<double>> GetRoutes(const RoutingIndexManager &manager, const RoutingModel &routing, const Assignment &solution)
  {
    std::vector<std::vector<double>> routes(manager.num_vehicles());
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

  std::vector<std::vector<double>> Tsp(std::vector<std::vector<double>> distance_matrix)
  {
    DataModel data;
    data.distance_matrix = distance_matrix;
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
  std::vector<std::vector<double>> distance_matrix;
  std::vector<double> row;

  std::string line;
  while (std::getline(std::cin, line, ',') && !line.empty())
  {
    if (line == "___")
    {
      distance_matrix.push_back(row);
      row.clear();
      continue;
    }
    double value = std::stod(line);
    row.push_back(value);
  }

  std::vector<std::vector<double>> routes = operations_research::Tsp(distance_matrix);
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