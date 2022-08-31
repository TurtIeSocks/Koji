/*
 * AUTHOR: ghoshanirban
 * SOURCE: https://github.com/ghoshanirban/UnitDiskCoverAlgorithms/blob/main/LL2014.h
 */

#ifndef LIULU_H
#define LIULU_H

#include <chrono>
#include <list>
#include <vector>
#include <fstream>
#include <iostream>

#include <CGAL/Cartesian.h>
typedef CGAL::Cartesian<double>::Point_2 Point;
typedef CGAL::Cartesian<double>::Segment_2 Segment;

typedef std::pair<int, int> intPair;
typedef std::unordered_map<intPair, int, boost::hash<intPair>> HashMap;

class LL2014
{
    std::vector<Point> &P;
    std::list<Point> &diskCenters;

    const long double sqrt3 = std::sqrt(3), sqrt3Over2 = sqrt3 / 2;

    struct sortByX
    {
        bool operator()(const Point &p1, const Point &p2)
        {
            return (p1.x() < p2.x());
        }
    };

public:
    LL2014(std::vector<Point> &P, std::list<Point> &diskCenters) : P(P), diskCenters(diskCenters) {}

    double execute()
    {
        assert(!P.empty());

        auto start = std::chrono::high_resolution_clock::now();

        unsigned answer = P.size() + 1;
        std::sort(P.begin(), P.end(), sortByX());

        HashMap H;
        for (unsigned i = 0; i < 6; i++)
        {

            unsigned current = 0;
            long double rightOfCurrentStrip = P[0].x() + ((i * sqrt3) / 6);
            std::list<Point> tempC;

            while (current < P.size())
            {

                if (P[current].x() > rightOfCurrentStrip)
                {
                    int jump = (int)((P[current].x() - rightOfCurrentStrip) / sqrt3);
                    rightOfCurrentStrip += jump * sqrt3;

                    if (jump > 0)
                        continue;
                }

                unsigned indexOfTheFirstPointInTheCurrentStrip = current;

                while (current < P.size() && P[current].x() < rightOfCurrentStrip)
                    current++;

                std::vector<Segment> segments;
                segments.reserve(current - indexOfTheFirstPointInTheCurrentStrip + 1);
                long double xOfRestrictionline = rightOfCurrentStrip - sqrt3Over2;

                for (unsigned j = indexOfTheFirstPointInTheCurrentStrip; j < current; j++)
                {
                    long double distanceFromRestrictionLine = P[j].x() - xOfRestrictionline;
                    long double y = CGAL::sqrt(1 - (distanceFromRestrictionLine * distanceFromRestrictionLine));
                    segments.emplace_back(Segment(Point(xOfRestrictionline, P[j].y() + y), Point(xOfRestrictionline, P[j].y() - y)));
                }

                rightOfCurrentStrip += sqrt3;

                if (segments.empty())
                    continue;

                std::sort(segments.begin(), segments.end(), [](const Segment &si, const Segment &sj)
                          { return (si.target().y() > sj.target().y()); });

                long double lowestY = segments[0].target().y();

                for (unsigned k = 1; k < segments.size(); k++)
                    if (segments[k].source().y() < lowestY)
                    {
                        tempC.emplace_back(Point(xOfRestrictionline, lowestY));
                        lowestY = segments[k].target().y();
                    }

                tempC.emplace_back(Point(xOfRestrictionline, lowestY));
            }

            if (tempC.size() < answer)
            {
                answer = tempC.size();
                diskCenters.clear();

                for (const Point &p : tempC)
                    diskCenters.push_back(p);
            }
        }

        auto stop = std::chrono::high_resolution_clock::now();
        std::chrono::duration<double> duration = stop - start;
        return duration.count();
    }
};

#endif // LIULU_H
