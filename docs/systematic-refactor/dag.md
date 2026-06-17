# Koji dep DAG (low → high). Symbol can move to T iff every consumer depends on T.

Tier 0 leaves (no internal deps): macros, koji-jobs, migration, nominatim
Tier 1: koji-core -> macros
Tier 2 -> koji-core: koji-db, koji-dragonite, koji-events, koji-plugins, koji-scanner
Tier 3: algorithms -> koji-core (+koji-plugins optional), macros
Tier 4: koji-service -> [koji-core,scanner,db,plugins,jobs,events,dragonite,algorithms,migration,nominatim]
        koji-wasm -> algorithms, koji-core
Bins (sinks): koji-cli -> [db,jobs,service]; koji-server -> [service]

# Lowest-common-ancestor targets for common consumer sets:
# {algorithms,service}              -> algorithms
# {algorithms,service,wasm}         -> algorithms   (wasm->algorithms, service->algorithms)
# {db,service}                      -> koji-db
# {scanner,service}                 -> koji-scanner
# {plugins,service}                 -> koji-plugins
# {db,plugins,...} siblings         -> koji-core (no lower common ancestor)
# {events,service}                  -> koji-events
