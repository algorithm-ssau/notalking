from pathlib import Path
from vosk import Model, KaldiRecognizer
import pyaudio


def find_absolute_path():
    script_path = Path(__file__).resolve()
    absolute__path = script_path.__str__().split("\\")
    absolute__path = "\\".join(absolute__path[:len(absolute__path) - 1])
    return absolute__path


def get_recognizer(model_name="vosk-model-small-ru-0.22"):
    model_path = find_absolute_path() + f"\\models_for_vosk\\{model_name}"
    model = Model(model_path)
    sample_rate = 44100
    chunk_size = 4096
    p = pyaudio.PyAudio()
    stream = p.open(format=pyaudio.paInt16,
                    channels=1,
                    rate=sample_rate,
                    input=True,
                    input_device_index=1,
                    frames_per_buffer=chunk_size)
    stream.start_stream()
    recognizer = KaldiRecognizer(model, sample_rate)
    return recognizer
