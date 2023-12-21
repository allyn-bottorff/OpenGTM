// Copyright 2023 Allyn L. Bottorff
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
	"net"

	"github.com/coredns/coredns/plugin"
	clog "github.com/coredns/coredns/plugin/pkg/log"
	"github.com/coredns/coredns/request"

	"github.com/miekg/dns"
)

type Gtm struct {
	Next plugin.Handler
}

func (g *Gtm) ServeDNS(ctx context.Context, w dns.ResponseWriter, r *dns.Msg) (int, error) {

	pw := NewResponseHandler(w)

	return plugin.NextOrFailure(g.Name(), g.Next, ctx, pw, r)
}

func (g *Gtm) Name() string {
	return "gtm"
}

type ResponseHandler struct {
	dns.ResponseWriter
}

func (r *ResponseHandler) WriteMsg(res *dns.Msg) error {
	question := res.Question[0].String()

	clog.Info("example. Question: %s", question)
	clog.Info("responding with garbage")

	state := request.Request{
		W:   r.ResponseWriter,
		Req: res,
	}

	ip := "8.8.8.8"
	var rr dns.RR

	rr = new(dns.A)
	rr.(*dns.A).Hdr = dns.RR_Header{
		Name:   state.QName(),
		Rrtype: dns.TypeA,
		Class:  state.QClass(),
	}
	rr.(*dns.A).A = net.ParseIP(ip)

	res.Answer = []dns.RR{rr}

	return r.ResponseWriter.WriteMsg(res)
}

func NewResponseHandler(w dns.ResponseWriter) *ResponseHandler {
	return &ResponseHandler{ResponseWriter: w}
}
