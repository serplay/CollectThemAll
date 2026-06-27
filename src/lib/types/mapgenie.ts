export interface Game {
  id: number;
  title: string;
  status: string;
  image: string | null;
  logo: string | null;
  config: GameConfig;
  maps: Map[];
}

export interface GameConfig {
    cdn_url: string;
    tiles_base_url: string;
    presets_enabled: boolean;
    marker_sprite_url: string;
    compass_enabled: boolean;
    heatmaps_enabled: boolean;
}

export interface Map {
    id: number;
    game_id: number;
    title: string;
    slug: string;
    image: string;
    order: number;
    enabled: boolean;
    available: boolean;
    work_in_progress: boolean;
    initial_zoom: number;
    initial_latitude: number;
    initial_longitude: number;
    locations_count: number;
    map_style: null;
}