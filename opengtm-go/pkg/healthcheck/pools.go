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
	"crypto/tls"
	"fmt"
	"io"
	"log"
	"math/rand/v2"
	"net"
	"net/http"
	"net/netip"
	"strings"
	"time"
)

type HTTPPool struct {
	CommonPool
	Send                 string `json:"send"`
	HostHeader           string `json:"host_header"` // string value of the Host header to send. Empty string indicates no host header
	HTTPSEnabled         bool   `json:"https_enabled"`
	HTTPSRequireValidity bool   `json:"https_require_validity"`
	ReceiveUpString      string `json:"receive_up_string"` // String value to look for in response body. Empty string indicates no string checking
	ReceiveUpCodes       []int  `json:"receive_up_codes"`  // HTTP response codes. A match on these superceeds any string checking
}
type TCPPool struct {
	CommonPool
}

type CommonPool struct {
	Name             string     `json:"name"`
	Port             uint       `json:"port"`
	Members          []string   `json:"members"` // Could be either IP addresses or DNS names
	FallbackIP       netip.Addr `json:"fallback_ip"`
	Interval         uint       `json:"interval"`
	FailureThreshold int        `json:"failure_threshold"`
}

// long-lived TCP poller which loops and sleeps based on the configured interval
func (p *TCPPool) Poller(ctx context.Context, host string, table *HealthTable) {

	// Set a random backoff timer between 0 and the interval. Sleep the duration
	// of the backoff timer and then start the polling loop

	log.Printf("INFO: Starting poller for %s: %d\n", host, p.Port)
	backoff := rand.Int() % int(p.Interval)
	log.Printf("INFO: Waiting %d seconds before starting poll for %s: %d\n", backoff, host, p.Port)
	time.Sleep(time.Duration(backoff) * time.Second)
	socket := fmt.Sprintf("%s:%d", host, p.Port)

	for {
		select {
		case <-ctx.Done(): // catch cancel signal and exit
			return
		default:
		}

		// Resolve DNS for the host. Takes a host name or an IP address
		var ips, err = net.LookupIP(host)
		if err != nil {
			// DNS resolution failed
			// TODO: (alb) handle this
		}
		if len(ips) > 0 { // TODO: (alb) handle the case when the IP list is 0
			ip, err := netip.ParseAddr(ips[0].String())
			if err != nil {
				// this should not be possible.
				ip = netip.MustParseAddr("127.0.0.1")
			}
			member := Member{
				Host: host,
				Ip:   ip,
			}
			conn, err := net.Dial("tcp", socket)
			if err != nil {
				member.Healthy = false
			} else {
				conn.Close() // techinically this can return an error
				member.Healthy = true
			}
			table.setHealth(&p.CommonPool, member)
		}
		select {
		case <-ctx.Done(): // catch cancel signal and exit
			return
		default:
		}
		time.Sleep(time.Duration(p.Interval) * time.Second)
	}
}

// Long lived HTTP poller which loops and sleeps based on configured interval
func (p *HTTPPool) Poller(ctx context.Context, host string, table *HealthTable) {

	// Set a random backoff timer between 0 and the interval. Sleep the duration
	// of the backoff timer and then start the polling loop
	log.Printf("INFO: Starting poller for %s: %d\n", host, p.Port)
	backoff := rand.Int() % int(p.Interval)
	log.Printf("INFO: Waiting %d seconds before starting poll for %s: %d\n", backoff, host, p.Port)
	time.Sleep(time.Duration(backoff) * time.Second)

	tlsConfig := &tls.Config{InsecureSkipVerify: !p.HTTPSRequireValidity}
	tr := &http.Transport{TLSClientConfig: tlsConfig}
	tr.DisableKeepAlives = true

	client := &http.Client{Transport: tr}

PollLoop:
	for {
		// tr.CloseIdleConnections()
		select {
		case <-ctx.Done(): // catch cancel signal and exit
			return
		default:
		}
		time.Sleep(time.Duration(p.Interval) * time.Second)
		select {
		case <-ctx.Done(): // catch cancel signal and exit
			return
		default:
		}
		ips, err := net.LookupIP(host)
		if err != nil {
			continue
		}
		if len(ips) > 0 {
			ip, err := netip.ParseAddr(ips[0].String())
			if err != nil {
				// this should not be possible.
				ip = netip.MustParseAddr("127.0.0.1")
			}
			member := Member{
				Host: host,
				Ip:   ip,
			}
			// Construct the url based on the pool settings
			var url string
			if p.HTTPSEnabled == true {
				url = fmt.Sprintf("https://%s:%d%s", host, p.Port, p.Send)
			} else {
				url = fmt.Sprintf("http://%s:%d%s", host, p.Port, p.Send)
			}

			// Make the HTTP call
			req, err := http.NewRequest("GET", url, nil)
			if p.HostHeader != "" {
				req.Header.Add("Host", p.HostHeader)
			}
			if err != nil {
				log.Printf("ERROR: creating request for %s. %s\n", url, err)
			}
			resp, err := client.Do(req)
			if err != nil {
				log.Printf("INFO: Error calling %s. %s\n", url, err)
				member.Healthy = false
				table.setHealth(&p.CommonPool, member)
				continue
			}

			// Check for status codes. Status code match superceeds any other
			// matches. Check for this first and skip the rest if there is a
			// match
			for _, c := range p.ReceiveUpCodes {
				if resp.StatusCode == c {
					member.Healthy = true
					table.setHealth(&p.CommonPool, member)
					resp.Body.Close()
					continue PollLoop
				}
			}
			if len(p.ReceiveUpString) == 0 {
				member.Healthy = false
				table.setHealth(&p.CommonPool, member)
				resp.Body.Close()
				continue PollLoop
			}

			respBytes, err := io.ReadAll(resp.Body)
			if err != nil {
				member.Healthy = false
				table.setHealth(&p.CommonPool, member)
				resp.Body.Close()
				continue PollLoop
			}
			resp.Body.Close()

			if strings.Contains(string(respBytes), p.ReceiveUpString) {
				member.Healthy = true
				table.setHealth(&p.CommonPool, member)
				resp.Body.Close()
				continue PollLoop
			}
			resp.Body.Close()

			// At this point all attempts to prove the member is up have failed.
			member.Healthy = false
			table.setHealth(&p.CommonPool, member)
		}
	}

}
