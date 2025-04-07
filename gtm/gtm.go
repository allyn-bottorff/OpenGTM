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

package gtm

import (
	"context"
	"fmt"
	"io"
	"net"
	"net/http"
	"strings"

	"github.com/coredns/caddy"
	"github.com/coredns/coredns/core/dnsserver"
	"github.com/coredns/coredns/plugin"
	clog "github.com/coredns/coredns/plugin/pkg/log"

	"github.com/miekg/dns"

	"github.com/coredns/coredns/request"
)

var log = clog.NewWithPlugin("gtm")

type Gtm struct {
	Next plugin.Handler
}
type ResponseHandler struct {
	dns.ResponseWriter
}

func (g Gtm) ServeDNS(ctx context.Context, w dns.ResponseWriter, r *dns.Msg) (int, error) {

	question := r.Question[0].Name

	// Remove the technically correct (but not usually seen) trailing dot which
	// reprents that the domain name is fully qualified to the root zone.
	// The healthchecker API doesn't expect there to be trailing "."
	question = strings.TrimSuffix(question, ".")

	log.Infof("Question: %s", question)

	state := request.Request{
		W:   w,
		Req: r,
	}

	var rr dns.RR

	rr = new(dns.A)
	rr.(*dns.A).Hdr = dns.RR_Header{
		Name:   state.QName(),
		Rrtype: dns.TypeA,
		Class:  state.QClass(),
	}

	// req.Answer = []dns.RR{rr}

	url := fmt.Sprintf("http://127.0.0.1:8080/info?name=%s", question)
	resp, err := http.Get(url)
	if err != nil {
		log.Error("Call to healthchecker failed.")
		return dns.RcodeServerFailure, err

	}
	defer resp.Body.Close()
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		log.Error("Unable to parse response from healthchecker.")
		return dns.RcodeServerFailure, err
	}

	rr.(*dns.A).A = net.ParseIP(string(body))

	reply := &dns.Msg{}
	reply.SetReply(r)
	reply.Authoritative = true

	reply.Extra = []dns.RR{rr}

	w.WriteMsg(reply)

	return plugin.NextOrFailure(g.Name(), g.Next, ctx, w, r)
}

func (g Gtm) Name() string {
	return "gtm"
}

func NewResponseHandler(w dns.ResponseWriter) *ResponseHandler {
	return &ResponseHandler{ResponseWriter: w}
}

func (g Gtm) Ready() bool {
	return true
}

func setup(c *caddy.Controller) error {
	c.Next()
	if c.NextArg() {
		return plugin.Error("gtm", c.ArgErr())
	}

	dnsserver.GetConfig(c).AddPlugin(func(next plugin.Handler) plugin.Handler {
		return Gtm{Next: next}
	})

	return nil
}

func init() {
	plugin.Register("gtm", setup)
}
