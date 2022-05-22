from constants import *
from keras.models import load_model
from glob import glob
from random import shuffle
import numpy as np
import soundfile as sf

USE_FILES = 5

files = glob("./samples/*.npy")
files.sort()
shuffle(files)
files = files[:USE_FILES]

choose = glob("model_*")
choose.sort()
choose = choose[-1]
print(f"loading model {choose}")
autoenc = load_model(choose)

lis = []
for x in files:
    lis += [np.load(x)]
lis = np.asarray(lis).reshape(USE_FILES, AUDIO_LEN, 1)
for e, sl in enumerate(lis):
    sf.write(f"x_{e}.wav", (sl.reshape(AUDIO_LEN) * 2) - 1, SAMPLING)
out = autoenc.predict(lis)
for e, sl in enumerate(out):
    sf.write(f"y_{e}.wav", (sl.reshape(AUDIO_LEN) * 2) - 1, SAMPLING)


