from constants import *
import tensorflow.keras
from keras.models import Model
from keras.layers import Input, Conv1D as CL, UpSampling1D as up
from sys import exit

FIL = [1024, 256, 64]
KER = [16, 32, 64]
SIS = [16, 5, 5]

if len(FIL) != len(KER) or len(KER) != len(SIS):
    print("Layers must be equal.")
    exit(1)
LAY = len(FIL) - 1

encinp = Input((AUDIO_LEN,1))
enc = None
for x in range(LAY + 1):
    if enc == None:
        enc = CL(FIL[x], KER[x], strides=SIS[x], padding="same")(encinp)
    else:
        enc = CL(FIL[x], KER[x], strides=SIS[x], padding="same")(enc)
enc = CL(1, 1, padding="same")(enc)
next_shape = Model(encinp, enc).output_shape[1:]

decinp = Input(next_shape)
dec = None
for x in range(LAY + 1):
    if dec == None:
        dec = CL(FIL[LAY - x], KER[LAY - x], padding="same")(decinp)
    else:
        dec = CL(FIL[LAY - x], KER[LAY - x], padding="same")(dec)
    dec = up(SIS[LAY - x])(dec)
dec = CL(1,1, padding="same")(dec)

encoder = Model(encinp, enc, name="enc")
encoder.summary()
encoder.save("model_enc_000")

decoder = Model(decinp, dec, name="dec")
decoder.summary()
decoder.save("model_dec_000")


