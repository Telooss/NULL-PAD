/* Codes NULL/PAD partages avec le moteur (engine/src/codes.rs).
 * engine/tests/firmware_codes.rs verifie que ce fichier reste coherent. */
#pragma once

#include <dt-bindings/zmk/hid_usage.h>
#include <dt-bindings/zmk/hid_usage_pages.h>

#define NP_ACTION_BASE 0x0500
#define NP_PROFILE_STRIDE 32
#define NP_ANNOUNCE_BASE 0x0600
#define ACT(p, i) ZMK_HID_USAGE(HID_USAGE_CONSUMER, (NP_ACTION_BASE + ((p) - 1) * NP_PROFILE_STRIDE + (i)))
#define ANN(p) ZMK_HID_USAGE(HID_USAGE_CONSUMER, (NP_ANNOUNCE_BASE + (p)))

/* Entree = position de touche du firmware = InputId du moteur. */
#define IN_K00 0
#define IN_K01 1
#define IN_K02 2
#define IN_K03 3
#define IN_K04 4
#define IN_K05 5
#define IN_K06 6
#define IN_K07 7
#define IN_K08 8
#define IN_K09 9
#define IN_K10 10
#define IN_K11 11
#define IN_K12 12
#define IN_K13 13
#define IN_K14 14
#define IN_K15 15
#define IN_K16 16
#define IN_K17 17
#define IN_K18 18
#define IN_K19 19
#define IN_M1 20
#define IN_M2 21
#define IN_M3 22
#define IN_ENC_L_CW 23
#define IN_ENC_L_CCW 24
#define IN_ENC_L_PRESS 25
#define IN_ENC_R_CW 26
#define IN_ENC_R_CCW 27
#define IN_ENC_R_PRESS 28
