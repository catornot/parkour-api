# API Reference

All endpoints require an `authentication` header

```
authentication: your_secret_here
```

---

## Maps

### List maps

```
GET /v1/maps
```

Returns an ordered list of all map names.

**Response `200`**
```json
["mp_lf_traffic", "mp_thaw"]
```

---

### Create map

```
POST /v1/maps
```

**Body**
```json
{
    "map_name": "mp_lf_traffic"
}
```

**Responses**
| Status | Meaning |
|--------|---------|
| `201`  | Map created |
| `208`  | Map already exists |

---

## Routes

### List routes for a map

```
GET /v1/maps/:map_name/routes
```

Returns all routes registered on the map.

**Response `200`**: array of route objects (same shape as the create body, see below)

**Response `404`**: map not found

---

### Create route

```
POST /v1/maps/:map_name/routes
```

**Body**

All coordinate arrays are `[x, y, z]` (floats). Angle arrays are `[pitch, yaw, roll]` (integers unless noted). Dimension arrays are `[width, height]` (integers).

```json
{
    "name": "Example Route",

    "start_line": {
        "origin":     [x, y, z],
        "angles":     [p, y, r],
        "dimensions": [w, h],
        "trigger":    [[x1, y1, z1], [x2, y2, z2]]
    },

    "finish_line": {
        "origin":     [x, y, z],
        "angles":     [p, y, r],
        "dimensions": [w, h],
        "trigger":    [[x1, y1, z1], [x2, y2, z2]]
    },

    "leaderboards": {
        "local": {
            "origin":     [x, y, z],
            "angles":     [p, y, r],
            "dimensions": [w, h],
            "source": {
                "origin":     [x, y, z],
                "angles":     [p, y, r],
                "dimensions": [w, h]
            }
        },
        "world": {
            "origin":     [x, y, z],
            "angles":     [p, y, r],
            "dimensions": [w, h],
            "source": {
                "origin":     [x, y, z],
                "angles":     [p, y, r],
                "dimensions": [w, h]
            }
        }
    },

    "checkpoints": [
        [x, y, z]
    ],

    "start": {
        "origin": [x, y, z],
        "angles": [p, y, r]
    },

    "end": {
        "origin": [x, y, z]
    },

    "ziplines": [
        [[x1, y1, z1], [x2, y2, z2]]
    ],

    "robot": {
        "origin":          [x, y, z],
        "angles":          [p, y, r],
        "talkable_radius": 60,
        "animation":       "mv_idle_weld"
    },

    "indicator": {
        "coordinates":    [x, y, z],
        "trigger_radius": 400
    },

    "route_name": {
        "origin":     [x, y, z],
        "angles":     [p, y, r],
        "dimensions": [w, h]
    },

    "perks":    {},
    "entities": []
}
```

`perks` and `entities` are optional; they default to `{}` and `[]` respectively.

Each entity in `entities` has the shape:
```json
{
    "coordinates": [x, y, z],
    "angles":      [x, y, z],
    "scale":       1.0,
    "model_name":  "mdl/some_model.mdl",
    "hidden":      false
}
```
`hidden` is optional.

**Responses**
| Status | Meaning |
|--------|---------|
| `201`  | Route created. Response body is the route's slug (e.g. `"example-route"`) |
| `208`  | Route name already used on this map |
| `404`  | Map not found |

**Slugification**: the route slug is derived from its name by lowercasing and replacing any non-alphanumeric characters with hyphens. `"My Cool Route"` becomes `"my-cool-route"`. The slug is used in score URLs and the scoreboard.

---

## Scores

### List scores for a route

```
GET /v1/maps/:map_name/routes/:route_slug/scores
```

Returns scores sorted by time ascending.

**Response `200`**
```json
[
    { "uid": "abc123", "name": "PlayerOne", "time": 28.441 },
    { "uid": "def456", "name": "PlayerTwo", "time": 31.002 }
]
```

**Response `404`**: map or route not found

---

### Submit a score

```
POST /v1/maps/:map_name/routes/:route_slug/scores
```

Only keeps the personal best per player. If the submitted time is not an improvement, the entry is rejected. Player names are updated automatically on each submission, so name changes propagate without any extra calls.

**Body**
```json
{
    "uid":  "abc123",
    "name": "PlayerOne",
    "time": 28.441
}
```

`time` is in seconds (float).

**Responses**
| Status | Meaning |
|--------|---------|
| `201`  | Score recorded |
| `208`  | A better (or equal) time already exists for this player |
| `404`  | Map or route not found |

---

## Scoreboard

```
GET /
```

Web scoreboard showing all maps and routes. Not authentication-protected. Static assets are served from `/assets/`.
