import time
import os

log_file_path = "blockchain_metrics.log"

def follow(log_file):
    log_file.seek(0, os.SEEK_END)
    while True:
        line = log_file.readline()
        if not line:
            time.sleep(0.1) 
            continue
        yield line

def display_dashboard():
    with open(log_file_path, "r") as log_file:
        log_lines = follow(log_file)
        print("Blockchain Metrics Dashboard")
        print("---------------------------")
        for line in log_lines:
            print(line.strip())  

if __name__ == "__main__":
    display_dashboard()
