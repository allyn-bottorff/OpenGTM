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
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"net/netip"
	"strings"
	"sync"
)

type HealthTable struct {
	mu     sync.RWMutex
	Health map[string][]Member
}

func NewHealthTable() *HealthTable {
	var t = new(HealthTable)
	t.mu.Lock()
	defer t.mu.Unlock()
	t.Health = make(map[string][]Member)
	return t
}

// format the health table as json and return it as a string
func (t *HealthTable) dumpTable() (string, error) {
	t.mu.RLock()
	defer t.mu.RUnlock()
	var tableString, err = json.Marshal(t.Health)
	if err != nil {
		return "", err
	}
	return string(tableString), nil
}

// add a new, empty pool to the table
func (t *HealthTable) AddPool(name string) {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.Health[name] = make([]Member, 0, 2)
}

func (t *HealthTable) BuildFromConfig(config *Config) {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.Health = make(map[string][]Member)

	for _, p := range config.TcpPools {
		var members []Member
		for _, h := range p.Members {
			members = append(members, Member{Host: h, Ip: p.FallbackIP, Healthy: false})
		}
		t.Health[p.Name] = members
	}
}

// Return a JSON formatted string representation of the health table
func (t *HealthTable) HandleGet(w http.ResponseWriter, r *http.Request) {
	var tableString, err = t.dumpTable()
	if err != nil {
		http.Error(w, "Unable to marshal health table to JSON", http.StatusInternalServerError)
		log.Println("Error: Unable to marshal health table to JSON")
		return
	}
	fmt.Fprint(w, tableString)
}

// Return a single IP address from a pool
func (t *HealthTable) HandleGetIP(w http.ResponseWriter, r *http.Request) {
	pathParts := strings.Split(r.URL.Path, "/")
	var poolName string
	if len(pathParts) < 2 {
		log.Printf("bad url: %s\n", r.URL.Path)
	} else if len(pathParts) > 3 {
		// TODO: (alb) return error
		log.Printf("bad url: %s\n", r.URL.Path)
	} else {
		poolName = pathParts[2]
	}

	ip := getIPFromPoolGA(t, poolName)

	fmt.Fprintf(w, "%s", ip.String())
}

// get the first healthy host
// if no healthy hosts, return the first host in the list
func getIPFromPoolGA(t *HealthTable, pool string) netip.Addr {
	t.mu.RLock()
	defer t.mu.RUnlock()

	for _, m := range t.Health[pool] {
		if m.Healthy == true {
			return m.Ip
		}
	}
	return t.Health[pool][0].Ip
	// TODO: (alb) check for pool length or return fallback instead. or return
	// an error or something like that
}

// describes details for a single pool member and is the main way state is
// shared between pollers and the API
type Member struct {
	Host     string     `json:"host"`     // could be a host name or IP address
	Ip       netip.Addr `json:"ip"`       // resolved IP address of the host
	Healthy  bool       `json:"healthy"`  // up/down status of the host
	Failures int        `json:"failures"` // running count of consecutive failed checks
}

// set the health of a pool member. Can be blocked by waiting on a mutex
func (t *HealthTable) setHealth(pool *CommonPool, member Member) {
	t.mu.RLock()

	log.Printf("Setting %s: %s health to: %v\n", pool.Name, member.Host, member.Healthy)
	found := false
	var idx int
	for i := range t.Health[pool.Name] {
		if t.Health[pool.Name][i].Host == member.Host {
			idx = i
			found = true
			break
		}
	}

	var needsWrite bool = false
	if found == true {
		if t.Health[pool.Name][idx].Ip != member.Ip {
			needsWrite = true
		}
		if t.Health[pool.Name][idx].Healthy != member.Healthy {
			needsWrite = true
		}
		if member.Healthy == false && t.Health[pool.Name][idx].Failures <= pool.FailureThreshold {
			needsWrite = true
		}
	} else {
		needsWrite = true
	}

	if needsWrite == true {
		t.mu.RUnlock()
		t.mu.Lock()
		defer t.mu.Unlock()
		if found == true {
			if member.Healthy == true {
				// health check has passed; reset the failure count
				member.Failures = 0
			} else {
				// in this case the health check has failed. Increment the failure
				// count and check if the count is greater than or equal to the
				// threshold. Only set the table's health status to false if the
				// failure count exceeded the threshold.
				member.Failures = t.Health[pool.Name][idx].Failures + 1
				if member.Failures >= pool.FailureThreshold {
					member.Failures = pool.FailureThreshold
					member.Healthy = false
				} else {
					member.Healthy = true
				}
			}
			t.Health[pool.Name][idx] = member
		} else {
			t.Health[pool.Name] = append(t.Health[pool.Name], member)
		}
	} else {
		t.mu.RUnlock()
	}
}
