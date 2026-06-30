// Turns raw location records into GeoJSON that MapLibre can draw.
//
// Cybersecurity note: the `locations` we receive came from a downloaded JSON
// file, so we do not blindly trust it. We filter out anything without real
// coordinates before mapping it, and we serialise the nested `media` array into a
// string so it travels safely as a GeoJSON feature property.

/** The exact shape MapLibre's geojson source expects. */
export type LocationFeatureCollection = {
  type: 'FeatureCollection';
  features: any[];
};

/**
 * Build the feature collection for all locations, stamping each feature with a
 * `found` flag (1/0) based on the player's found-set.
 */
export function buildLocationGeoJson(
  locations: any[],
  foundIds: Set<number>,
): LocationFeatureCollection {
  return {
    type: 'FeatureCollection',
    features: locations
      // Skip locations missing latitude/longitude — bad/partial data, not drawable.
      .filter((loc: any) => loc.latitude && loc.longitude)
      .map((loc: any) => ({
        type: 'Feature' as const,
        geometry: {
          type: 'Point' as const,
          coordinates: [parseFloat(loc.longitude), parseFloat(loc.latitude)],
        },
        properties: {
          id: loc.id,
          title: loc.title,
          category_id: loc.category_id,
          description: loc.description ?? '',
          media: JSON.stringify(loc.media ?? []),
          found: foundIds.has(loc.id) ? 1 : 0,
        },
      })),
  };
}

/** Build the feature collection for the player's own custom markers. Unlike the
 *  downloaded MapGenie locations, latitude/longitude here are already numbers
 *  (they came straight out of SQLite, not parsed from untrusted JSON text). */
export function buildCustomMarkerGeoJson(markers: {
  id: number;
  latitude: number;
  longitude: number;
  title: string;
  description: string;
}[]): LocationFeatureCollection {
  return {
    type: 'FeatureCollection',
    features: markers.map((m) => ({
      type: 'Feature' as const,
      geometry: {
        type: 'Point' as const,
        coordinates: [m.longitude, m.latitude],
      },
      properties: {
        id: m.id,
        title: m.title,
        description: m.description,
      },
    })),
  };
}
