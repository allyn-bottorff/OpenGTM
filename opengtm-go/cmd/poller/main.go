// Copyright 2025 Allyn L. Bottorff
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"net/http"
	_ "net/http/pprof"
	"sync"

	hc "github.com/allyn-bottorff/OpenGTM/opengtm-go/pkg/healthcheck"
)

func main() {
	// Set up CLI flags
	var configArg = flag.String("config", "", "path to the configuration file")
	var printConfig = flag.Bool("print-config", false, "dump the config and exit")
	flag.Parse()

	// Initialize the health table. After this, always modify the existing one
	var healthTable = new(hc.HealthTable)

	// TODO: (alb) make this optional based on a cli flag
	var config = new(hc.Config)

	if *configArg == "" {
		log.Println("No config file found. Starting with default config.")
		config.Default()
	} else {
		config.FromFile(*configArg)
	}

	if *printConfig == true {
		fmt.Printf("%v\n", config)
		return
	}
	healthTable.BuildFromConfig(config) // populate from the file-based config

	ctx, cancel := context.WithCancel(context.Background())
	config.Cancel = cancel

	// -----------------------------------------------------------------------
	// API SECTION
	// -----------------------------------------------------------------------

	// Kubernetes liveness route.
	http.HandleFunc("/livez", handleLivez)

	// Kubernetes readiness route.
	http.HandleFunc("/readyz", handleReadyz)

	// health table routes
	http.HandleFunc("GET /healthtable", healthTable.HandleGet)
	http.HandleFunc("GET /health/{pool}", healthTable.HandleGetIP)

	// configuration management routes
	http.HandleFunc("POST /config", config.HandlePost)
	http.HandleFunc("GET /config", config.HandleGet)
	http.HandleFunc("GET /cancel", config.HandleCancel)

	// Start the HTTP server
	go func() {
		var err = http.ListenAndServe("0.0.0.0:8080", nil)
		if err != nil {
			log.Fatalf("listen: %s\n", err)
		}
	}()

	// -----------------------------------------------------------------------
	//  POLLER SECTION
	// -----------------------------------------------------------------------

	for {
		// Build a wait group to catch the exit of pollers. This is necessary
		// because we need to be able to change the number of running pollers
		// based on a change in the configuration. Restart the pollers after
		// they exit.

		var wg = new(sync.WaitGroup)

		for _, p := range config.TcpPools {
			healthTable.AddPool(p.Name)

			for _, m := range p.Members {
				wg.Add(1)
				go func() {
					p.Poller(ctx, m, healthTable)
					wg.Done()
				}()
			}
		}

		for _, p := range config.HttpPools {
			healthTable.AddPool(p.Name)
			for _, m := range p.Members {
				wg.Add(1)
				go func() {
					p.Poller(ctx, m, healthTable)
					wg.Done()
				}()
			}
		}
		wg.Wait()
		log.Println("All pollers have exited")
		// refresh the context so that the cancel signal isn't still being sent
		ctx, cancel = context.WithCancel(context.Background())
		config.Cancel = cancel
	}

}

// Handle the liveness route
func handleLivez(w http.ResponseWriter, r *http.Request) {
	fmt.Fprint(w, "Healthy\n")
}

// Handle the readiness route
func handleReadyz(w http.ResponseWriter, r *http.Request) {
	fmt.Fprint(w, "Ready\n")
}
