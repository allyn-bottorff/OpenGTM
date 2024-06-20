package main

import (
	"errors"
	"net"
)

// Top level configuration
type Config struct {
	Pools []Pool `json:"pools"`
}

func (c *Config) validate() []error {
	var errors []error
	for _, p := range c.Pools {
		err := p.validate()
		if err != nil {
			errors = append(errors, err)
		}
	}
	return errors
}

// Configuration defining a single health checked pool
type Pool struct {
	Name        string       `json:"name"`
	Port        uint         `json:"port"`
	Members     []string     `json:"members"`
	FallbackIP  string       `json:"fallback_ip"`
	PollType    string       `json:"poll_type"`
	HttpOptions *HttpOptions `json:"http_options,omitempty"`
}

// Validate pool structure
func (p *Pool) validate() error {
	fallbackIP := net.ParseIP(p.FallbackIP)
	if fallbackIP == nil {
		return errors.New("Failed to parse Fallback IP")
	}
	switch p := p.PollType; p {
	case "HTTP":
	case "TCP":
	default:
		return errors.New("Poll type must be TCP or HTTP")
	}

	if p.PollType == "HTTP" && p.HttpOptions == nil {
		return errors.New("HTTP Options are required when Poll Type is HTTP")
	}
	if p.PollType == "HTTP" {
		err := p.HttpOptions.validate()
		return err
	}

	return nil
}

// Configuration options for HTTP type pollers
type HttpOptions struct {
	Send                 string      `json:"send"`
	HttpsEnabled         bool        `json:"https_enabled"`
	HttpsRequireValidity bool        `json:"https_require_validity"`
	ReceiveUp            HttpReceive `json:"receive_up"`
}

// Validate HTTPOptions
func (h *HttpOptions) validate() error {
	err := h.ReceiveUp.validate()
	return err
}

// HTTP receive options
type HttpReceive struct {
	StatusCodes *[]uint `json:"status_codes,omitempty"`
	String      *string `json:"string,omitempty"`
}

// Validate HttpReceive options
func (h *HttpReceive) validate() error {
	if h.String != nil && h.StatusCodes != nil {
		return errors.New("HttpReceive contains both String and StatusCode")
	}
	if h.String == nil && h.StatusCodes == nil {
		return errors.New("HttpReceive contains neither String nor StatusCode")
	}
	return nil
}
