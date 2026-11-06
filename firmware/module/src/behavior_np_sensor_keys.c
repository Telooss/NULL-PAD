/* Molette en touches cachees : chaque cran leve un appui puis un relachement
 * sur une position de touche (param1 = horaire, param2 = anti-horaire).
 * Calcul des crans repris de app/src/behaviors/behavior_sensor_rotate_common.c
 * (ZMK v0.3.0, MIT, The ZMK Contributors). */

#define DT_DRV_COMPAT zmk_behavior_np_sensor_keys

#include <zephyr/device.h>
#include <zephyr/drivers/sensor.h>
#include <zephyr/kernel.h>
#include <zephyr/logging/log.h>

#include <drivers/behavior.h>
#include <zmk/behavior.h>
#include <zmk/keymap.h>
#include <zmk/sensors.h>
#include <zmk/virtual_key_position.h>
#include <zmk/events/position_state_changed.h>

LOG_MODULE_DECLARE(zmk, CONFIG_ZMK_LOG_LEVEL);

struct np_sensor_keys_data {
    struct sensor_value remainder[ZMK_KEYMAP_SENSORS_LEN][ZMK_KEYMAP_LAYERS_LEN];
    int triggers[ZMK_KEYMAP_SENSORS_LEN][ZMK_KEYMAP_LAYERS_LEN];
};

static int np_sensor_keys_accept_data(struct zmk_behavior_binding *binding,
                                      struct zmk_behavior_binding_event event,
                                      const struct zmk_sensor_config *sensor_config,
                                      size_t channel_data_size,
                                      const struct zmk_sensor_channel_data *channel_data) {
    const struct device *dev = zmk_behavior_get_binding(binding->behavior_dev);
    struct np_sensor_keys_data *data = dev->data;

    const struct sensor_value value = channel_data[0].value;
    int triggers;
    int sensor_index = ZMK_SENSOR_POSITION_FROM_VIRTUAL_KEY_POSITION(event.position);

    /* Ancien format ec11 (crans dans val2), garde comme l'original. */
    if (value.val1 == 0) {
        triggers = value.val2;
    } else {
        struct sensor_value remainder = data->remainder[sensor_index][event.layer];

        remainder.val1 += value.val1;
        remainder.val2 += value.val2;
        remainder.val1 += remainder.val2 / 1000000;
        remainder.val2 %= 1000000;

        int trigger_degrees = 360 / sensor_config->triggers_per_rotation;
        triggers = remainder.val1 / trigger_degrees;
        remainder.val1 %= trigger_degrees;

        data->remainder[sensor_index][event.layer] = remainder;
    }

    data->triggers[sensor_index][event.layer] = triggers;
    return 0;
}

static int np_sensor_keys_process(struct zmk_behavior_binding *binding,
                                  struct zmk_behavior_binding_event event,
                                  enum behavior_sensor_binding_process_mode mode) {
    const struct device *dev = zmk_behavior_get_binding(binding->behavior_dev);
    struct np_sensor_keys_data *data = dev->data;

    const int sensor_index = ZMK_SENSOR_POSITION_FROM_VIRTUAL_KEY_POSITION(event.position);

    if (mode != BEHAVIOR_SENSOR_BINDING_PROCESS_MODE_TRIGGER) {
        data->triggers[sensor_index][event.layer] = 0;
        return ZMK_BEHAVIOR_TRANSPARENT;
    }

    int triggers = data->triggers[sensor_index][event.layer];
    uint32_t position;

    /* Meme sens que sensor_rotate : triggers > 0 = horaire = 1er parametre. */
    if (triggers > 0) {
        position = binding->param1;
    } else if (triggers < 0) {
        triggers = -triggers;
        position = binding->param2;
    } else {
        return ZMK_BEHAVIOR_TRANSPARENT;
    }

    /* La keymap indexe ses tableaux par position : hors keymap = refus. */
    if (position >= ZMK_KEYMAP_LEN) {
        LOG_ERR("np_enc: position %u hors keymap", position);
        return -EINVAL;
    }

    /* Evenement synchrone, comme le scan (physical_layouts.c) : combos et keymap le voient. */
    for (int i = 0; i < triggers; i++) {
        raise_zmk_position_state_changed(
            (struct zmk_position_state_changed){.source = ZMK_POSITION_STATE_CHANGE_SOURCE_LOCAL,
                                                .state = true,
                                                .position = position,
                                                .timestamp = k_uptime_get()});
        raise_zmk_position_state_changed(
            (struct zmk_position_state_changed){.source = ZMK_POSITION_STATE_CHANGE_SOURCE_LOCAL,
                                                .state = false,
                                                .position = position,
                                                .timestamp = k_uptime_get()});
    }

    return ZMK_BEHAVIOR_OPAQUE;
}

static const struct behavior_driver_api np_sensor_keys_driver_api = {
    .sensor_binding_accept_data = np_sensor_keys_accept_data,
    .sensor_binding_process = np_sensor_keys_process,
};

#define NP_SENSOR_KEYS_INST(n)                                                                     \
    static struct np_sensor_keys_data np_sensor_keys_data_##n = {};                                \
    BEHAVIOR_DT_INST_DEFINE(n, NULL, NULL, &np_sensor_keys_data_##n, NULL, POST_KERNEL,             \
                            CONFIG_KERNEL_INIT_PRIORITY_DEFAULT, &np_sensor_keys_driver_api);

DT_INST_FOREACH_STATUS_OKAY(NP_SENSOR_KEYS_INST)
