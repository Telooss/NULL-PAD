# Compile seulement si un noeud zmk,behavior-np-sensor-keys est actif (cf. Kconfig).
target_sources_ifdef(CONFIG_ZMK_BEHAVIOR_NP_SENSOR_KEYS app PRIVATE src/behavior_np_sensor_keys.c)
