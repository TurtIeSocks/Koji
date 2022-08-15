#ifndef FASTCOVERPP_H
#define FASTCOVERPP_H

#include <chrono>
#include <list>
#include <vector>
#include <unordered_set>

#include <CGAL/Cartesian.h>
typedef CGAL::Cartesian<double>::Point_2 Point;

class FASTCOVER_PP
{
    std::vector<Point> &P;
    std::list<Point> &diskCenters;

    const double sqrt2 = std::sqrt(2);
    const double additiveFactor = sqrt2 / 2;
    const double sqrt2TimesOnePointFiveMinusOne = (sqrt2 * 1.5) - 1;
    const double sqrt2TimesZeroPointFivePlusOne = (sqrt2 * 0.5) - 1;

    struct BoundingBox
    {
        double minX, maxX, minY, maxY;
        BoundingBox()
        {
            minX = minY = DBL_MAX;
            maxX = maxY = DBL_MIN;
        }
        explicit BoundingBox(const Point &p) : minX(p.x()), maxX(p.x()), minY(p.y()), maxY(p.y()) {}
        void update(const Point &p)
        {
            minX = std::min(minX, p.x());
            minY = std::min(minY, p.y());
            maxX = std::max(maxX, p.x());
            maxY = std::max(maxY, p.y());
        }
    };

    typedef std::pair<int, int> intPair;
    typedef std::pair<BoundingBox, bool> diskInfo;
    typedef std::unordered_map<intPair, diskInfo, boost::hash<intPair>> HashMap;

    inline bool static trytoMergeDisk(HashMap &H, HashMap::iterator &iterToSourceDisk, int vPrime, int hPrime, std::list<Point> &diskCenters)
    {
        auto iterToTargetDisk = H.find(std::make_pair(vPrime, hPrime));

        if (iterToTargetDisk == H.end())
            return false;

        if (iterToTargetDisk->second.second)
        {
            double minX = std::min((*iterToSourceDisk).second.first.minX, iterToTargetDisk->second.first.minX);
            double minY = std::min((*iterToSourceDisk).second.first.minY, iterToTargetDisk->second.first.minY);
            double maxX = std::max((*iterToSourceDisk).second.first.maxX, iterToTargetDisk->second.first.maxX);
            double maxY = std::max((*iterToSourceDisk).second.first.maxY, iterToTargetDisk->second.first.maxY);

            Point lowerLeft(minX, minY), upperRight(maxX, maxY);

            if (CGAL::squared_distance(lowerLeft, upperRight) <= 4)
            {
                (*iterToSourceDisk).second.second = false;
                iterToTargetDisk->second.second = false;
                diskCenters.push_back(CGAL::midpoint(lowerLeft, upperRight));
                return true;
            }
        }
        return false;
    }

public:
    FASTCOVER_PP(std::vector<Point> &P, std::list<Point> &diskCenters) : P(P), diskCenters(diskCenters) {}

    double execute()
    {
        assert(!P.empty());

        auto start = std::chrono::high_resolution_clock::now();

        HashMap H;

        for (const Point &p : P)
        {
            int v = floor(p.x() / sqrt2), h = floor(p.y() / sqrt2);
            double verticalTimesSqrtTwo = v * sqrt2, horizontalTimesSqrt2 = h * sqrt2;

            auto it = H.find(std::make_pair(v, h));
            if (it != H.end())
            {
                it->second.first.update(p);
                continue;
            }

            if ((p.x() >= verticalTimesSqrtTwo + sqrt2TimesOnePointFiveMinusOne))
            {
                it = H.find(std::make_pair(v + 1, h));
                if (it != H.end() && (CGAL::squared_distance(p, Point(sqrt2 * (v + 1) + additiveFactor, horizontalTimesSqrt2 + additiveFactor)) <= 1))
                {
                    it->second.first.update(p);
                    continue;
                }
            }

            if ((p.x() <= verticalTimesSqrtTwo - sqrt2TimesZeroPointFivePlusOne))
            {
                it = H.find(std::make_pair(v - 1, h));
                if (it != H.end() && (CGAL::squared_distance(p, Point(sqrt2 * (v - 1) + additiveFactor, horizontalTimesSqrt2 + additiveFactor)) <= 1))
                {
                    it->second.first.update(p);
                    continue;
                }
            }

            if ((p.y() <= horizontalTimesSqrt2 + sqrt2TimesOnePointFiveMinusOne))
            {
                it = H.find(std::make_pair(v, h - 1));
                if (it != H.end() && (CGAL::squared_distance(p, Point(verticalTimesSqrtTwo + additiveFactor, sqrt2 * (h - 1) + additiveFactor)) <= 1))
                {
                    it->second.first.update(p);
                    continue;
                }
            }

            if ((p.y() >= horizontalTimesSqrt2 - sqrt2TimesZeroPointFivePlusOne))
            {
                it = H.find(std::make_pair(v, h + 1));
                if (it != H.end() && (CGAL::squared_distance(p, Point(verticalTimesSqrtTwo + additiveFactor, sqrt2 * (h + 1) + additiveFactor)) <= 1))
                {
                    it->second.first.update(p);
                    continue;
                }
            }
            H[std::make_pair(v, h)] = std::make_pair(BoundingBox(p), true);
        }

        for (auto iter = H.begin(); iter != H.end(); ++iter)
        {
            int v = (*iter).first.first, h = (*iter).first.second;
            if (!(*iter).second.second)
            {
                continue;
            }

            // Attempt to merge with the S disk
            if (trytoMergeDisk(H, iter, v, h - 1, diskCenters))
                continue;

            // Attempt to merge with the N disk
            if (trytoMergeDisk(H, iter, v, h + 1, diskCenters))
                continue;

            // Attempt to merge with the E disk
            if (trytoMergeDisk(H, iter, v + 1, h, diskCenters))
                continue;

            // Attempt to merge with the W disk
            if (trytoMergeDisk(H, iter, v - 1, h, diskCenters))
                continue;

            // Attempt to merge with the SW disk
            if (trytoMergeDisk(H, iter, v - 1, h - 1, diskCenters))
                continue;

            // Attempt to merge with the SE disk
            if (trytoMergeDisk(H, iter, v + 1, h - 1, diskCenters))
                continue;

            // Attempt to merge with the NE disk
            if (trytoMergeDisk(H, iter, v + 1, h + 1, diskCenters))
                continue;

            // Attempt to merge with the NW disk
            if (trytoMergeDisk(H, iter, v - 1, h + 1, diskCenters))
                continue;
        }

        for (auto aPair : H)
            if (aPair.second.second)
                diskCenters.emplace_back(Point(aPair.first.first * sqrt2 + additiveFactor, aPair.first.second * sqrt2 + additiveFactor));

        auto stop = std::chrono::high_resolution_clock::now();
        std::chrono::duration<double> duration = stop - start;
        return duration.count();
    }
};

#endif // FASTCOVERPP_H
