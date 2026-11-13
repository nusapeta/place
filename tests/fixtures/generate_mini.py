#!/usr/bin/env python3
"""Generate the small PBF used by integration tests."""
import osmium
from osmium.osm import mutable

OUT = "tests/fixtures/mini.pbf"

def node(writer, node_id, lat, lon, **tags):
    writer.add_node(mutable.Node(id=node_id, location=(lon, lat), tags=tags))

def way(writer, way_id, refs, **tags):
    writer.add_way(mutable.Way(id=way_id, nodes=refs, tags=tags))

def relation(writer, rel_id, members, **tags):
    writer.add_relation(mutable.Relation(id=rel_id, members=members, tags=tags))

def member(ref, mtype, role=""):
    return osmium.osm.RelationMember(ref, mtype, role)

def ring(writer, first_way, first_node, lat1, lon1, lat2, lon2, tags):
    ids = [first_node, first_node + 1, first_node + 2, first_node + 3]
    node(writer, ids[0], lat1, lon1)
    node(writer, ids[1], lat1, lon2)
    node(writer, ids[2], lat2, lon2)
    node(writer, ids[3], lat2, lon1)
    way(writer, first_way, [ids[0], ids[1]], **tags)
    way(writer, first_way + 1, [ids[1], ids[2]], **tags)
    way(writer, first_way + 2, [ids[2], ids[3]], **tags)
    way(writer, first_way + 3, [ids[3], ids[0]], **tags)
    return [member(first_way + i, "w", "outer") for i in range(4)]

writer = osmium.SimpleWriter(OUT, overwrite=True)

# Named POIs, including a duplicate name and an English alias.
node(writer, 1, -6.2, 106.8166667, name="Jakarta", place="city", population="10562088",
     capital="yes", wikidata="Q3640", **{"name:en": "Jakarta"})
node(writer, 2, -6.2, 106.82, name="Indomaret", shop="convenience", brand="Indomaret")
node(writer, 3, -6.21, 106.83, name="Indomaret", shop="convenience", brand="Indomaret")
node(writer, 4, -6.205, 106.825, name="Alfamart", shop="convenience")
node(writer, 5, -6.215, 106.835, shop="convenience")
node(writer, 6, -6.19, 106.81, name="Masjid Contoh", amenity="place_of_worship",
     **{"name:en": "Example Mosque", "alt_name": "Masjid Lama"})

# Long road, more than one kilometre diagonal.
node(writer, 13, -6.2, 106.8)
node(writer, 14, -6.2, 106.95)
way(writer, 101, [13, 14], name="Jalan Panjang", highway="primary")

# Short road and a named building.
node(writer, 10, -6.200, 106.810)
node(writer, 11, -6.201, 106.811)
node(writer, 12, -6.202, 106.812)
way(writer, 100, [10, 11, 12], name="Jalan Merdeka", highway="residential")
node(writer, 15, -6.203, 106.813)
node(writer, 16, -6.2035, 106.8135)
node(writer, 17, -6.204, 106.814)
way(writer, 200, [15, 16, 17, 15], name="Gedung Contoh", building="yes")

# Way with a missing node reference; the builder must skip it and count the miss.
node(writer, 18, -6.206, 106.816)
way(writer, 102, [18, 99], name="Jalan Hilang", highway="service")

# Administrative rings, level 4 to 7, nested.
province = ring(writer, 300, 20, -6.30, 106.70, -6.10, 106.90,
                {})
city = ring(writer, 310, 30, -6.25, 106.75, -6.15, 106.85,
            {})
district = ring(writer, 320, 40, -6.22, 106.80, -6.18, 106.84,
                {})
village = ring(writer, 330, 50, -6.20, 106.81, -6.19, 106.82,
               {})
relation(writer, 1000, province, name="Daerah Khusus Jakarta", boundary="administrative",
         admin_level="4", **{"name:en": "Jakarta Special Region"})
relation(writer, 1001, city, name="Jakarta Pusat", boundary="administrative",
         admin_level="5", **{"name:en": "Central Jakarta"})
relation(writer, 1002, district, name="Gambir", boundary="administrative",
         admin_level="6")
relation(writer, 1003, village, name="Gambir Village", boundary="administrative",
         admin_level="7")

# Named relation that is not administrative.
node(writer, 60, -6.11, 106.71)
node(writer, 61, -6.11, 106.73)
node(writer, 62, -6.13, 106.73)
node(writer, 63, -6.13, 106.71)
way(writer, 400, [60, 61, 62, 63, 60], natural="coastline")
relation(writer, 1100, [member(400, "w", "outer")], name="Pulau Contoh",
         place="island", **{"name:en": "Example Island"})

writer.close()
print(OUT)
