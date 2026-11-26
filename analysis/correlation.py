import numpy as np
import pandas as pd
#import os
#from glob import glob

def autocorrelation():
    num = np.random.rand()
    return num

#Pulling in data to then send to various functions to start to build a correlation matrix
def main():
    #t, tvoc = pd.read_csv("../tools/logs/run_Car_400/tvoc.csv")
    t, tvoc = np.loadtxt("../tools/logs/run_Car_400/tvoc.csv",float,)
    #co2 = pd.read_csv("../tools/logs/run_Car_400/co2.csv")
    #ethanol = pd.read_csv("../tools/logs/run_Car_400/ethanol.csv")
    #h2 = pd.read_csv("../tools/logs/run_Car_400/h2.csv")
    print("-----------------------------------------------")
    print(t ," , ", tvoc[20])
    #print("-----------------------------------------------")
    #print(co2[co2_ppm])
    #print("-----------------------------------------------")
    #print(ethanol[ethanol_raw])
    #print("-----------------------------------------------")
    #print(h2[h2_raw])

if __name__ == "__main__":
    main()