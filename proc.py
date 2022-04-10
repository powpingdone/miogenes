import librosa
import numpy as np
from glob import glob
from sys import argv
from os import mkdir

mkdir("./tmp")

for audiofile in glob(argv[1] + "/*"):
    # need 6 seconds of audio, so use 48000 overlapping length
    # samples
    wav, _ = librosa.load(audiofile, sr=8000)
    
