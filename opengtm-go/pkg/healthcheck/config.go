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

package healthcheck

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"net/netip"
	"os"
)

// Top level configuration for health check pools
type Config struct {
	HttpPools []HTTPPool         `json:"http_pools"`
	TcpPools  []TCPPool          `json:"tcp_pools"`
	Cancel    context.CancelFunc `json:"-"`
}

// Populate the config from a file on disk
func (c *Config) FromFile(path string) {

	var configBytes, err = os.ReadFile(path)
	if err != nil {
		log.Println("Error: Failed to read config from file.")
	}
	err = json.Unmarshal(configBytes, c)
	if err != nil {
		log.Println("Error: Failed to parse config file.")
	}
}

// cancel the running pollers, causing them to exit
func (c *Config) HandleCancel(w http.ResponseWriter, r *http.Request) {
	c.Cancel()
	fmt.Fprint(w, "cancelling pollers\n")
}

// Accept a configuration from the API
func (c *Config) HandlePost(w http.ResponseWriter, r *http.Request) {
	var tempConfig = new(Config)
	var err = json.NewDecoder(r.Body).Decode(tempConfig)
	if err != nil {
		http.Error(w, "Unable to parse body", http.StatusBadRequest)
		log.Println("Error: Unable to parse body")
		return
	}
	c = tempConfig
	w.WriteHeader(http.StatusAccepted)
	fmt.Fprint(w, "OK\n")
}

// Return a configuration in a JSON string
func (c *Config) HandleGet(w http.ResponseWriter, r *http.Request) {
	var configString, err = json.Marshal(c)
	if err != nil {
		http.Error(w, "", http.StatusInternalServerError)
		log.Println("Error: Unable to marshal config to JSON")
		return
	}
	fmt.Fprint(w, string(configString))
}

// Replace a Config struct with safe default data
func (c *Config) Default() {
	var conf = new(Config)
	// conf.HttpPools = make([]HTTPPool, 1)
	tcpPool := TCPPool{
		CommonPool: CommonPool{
			Name:             "tcp-default",
			Port:             443,
			Members:          []string{"127.0.0.1"},
			FallbackIP:       netip.MustParseAddr("127.0.0.1"),
			Interval:         10,
			FailureThreshold: 3,
		},
	}
	// conf.HttpPools = make([]HTTPPool, 1)
	httpPool := HTTPPool{
		CommonPool: CommonPool{
			Name:             "http-default",
			Port:             443,
			Members:          []string{"127.0.0.1"},
			FallbackIP:       netip.MustParseAddr("127.0.0.1"),
			Interval:         10,
			FailureThreshold: 3,
		},
		Send:                 "/health",
		HostHeader:           "localhost",
		HTTPSEnabled:         false,
		HTTPSRequireValidity: false,
		ReceiveUpString:      "",
		ReceiveUpCodes:       []int{200, 201},
	}

	conf.HttpPools = append(conf.HttpPools, httpPool)
	conf.TcpPools = append(conf.TcpPools, tcpPool)
	*c = *conf
}
